mod servers_config;

use servers_config::{ServerConfig, ServerGroup};

use zellij_tile::prelude::*;

use std::collections::{BTreeMap, HashSet};
use std::path::PathBuf;

#[derive(Default)]
struct State {
    // the state of the plugin
    server_groups: Vec<ServerGroup>,
    current_selected_list_index: usize,
    selected_servers: HashSet<ServerConfig>,
    configuration: Configuration,
}

#[derive(Default)]
struct Configuration {
    layout: ZellijLayout,
    sync: bool,
    config_path: PathBuf,
}

#[derive(Default)]
enum ZellijLayout {
    #[default]
    Default,
    Compact,
}

register_plugin!(State);

// More info on plugins: https://zellij.dev/documentation/plugins

impl ZellijPlugin for State {
    fn load(&mut self, configuration: BTreeMap<String, String>) {
        request_permission(&[PermissionType::ChangeApplicationState]);

        self.configuration = Configuration::from_config(configuration);
        // self.server_groups = servers_config::load_config_from_path(&self.configuration.config_path);

        // TODO Remove test data

        self.server_groups = vec![
            ServerGroup::new(
                "dbs".to_string(),
                vec![
                    ServerConfig::new("db-01".to_string(), "k-db-01".to_string()),
                    ServerConfig::new("db-02".to_string(), "k-db-02".to_string()),
                ],
            ),
            ServerGroup::new(
                "deb11".to_string(),
                vec![
                    ServerConfig::new("deb11-01".to_string(), "deb11-prod01".to_string()),
                    ServerConfig::new("deb11-02".to_string(), "deb11-prod02".to_string()),
                ],
            ),
        ];

        eprintln!(
            "DEBUG toml:\n{}",
            toml::to_string_pretty(&self.server_groups).unwrap()
        );

        subscribe(&[EventType::Key]);
    }
    fn update(&mut self, event: Event) -> bool {
        let mut should_render = false;

        if let Event::Key(key) = event {
            match key.bare_key {
                BareKey::Down
                    if self.current_selected_list_index < self.get_displayed_lines_number() - 1 =>
                {
                    self.current_selected_list_index += 1;
                    should_render = true
                }
                BareKey::Up if self.current_selected_list_index > 0 => {
                    self.current_selected_list_index -= 1;
                    should_render = true
                }
                BareKey::Char(' ') => {
                    if let Some((group_idx, server_idx)) =
                        self.resolve_index(self.current_selected_list_index)
                    {
                        match server_idx {
                            Some(s_idx) => {
                                let server = &self.server_groups[group_idx].get_servers()[s_idx];
                                if !self.selected_servers.contains(server) {
                                    self.selected_servers.insert(server.clone());
                                } else {
                                    self.selected_servers.remove(server);
                                }
                            }
                            None => {
                                let group = &self.server_groups[group_idx];

                                if group.all_selected_in(&self.selected_servers) {
                                    for server in group.get_servers() {
                                        self.selected_servers.remove(server);
                                    }
                                } else {
                                    for server in self.server_groups[group_idx].get_servers() {
                                        self.selected_servers.insert(server.clone());
                                    }
                                }
                            }
                        }
                        should_render = true;
                    }
                }
                BareKey::Enter => {
                    let tab_name = "cssh";
                    let panes = self
                        .selected_servers
                        .iter()
                        .map(|server| {
                            format!(
                                r#"
                            ssh {{
                                args "{host}"
                            }}
                        "#,
                                host = server.get_host()
                            )
                        })
                        .collect::<Vec<String>>()
                        .join("\n");

                    let layout = self.configuration.layout.get_kdl(tab_name, &panes);
                    eprintln!("DEBUG layout:\n{}", layout);
                    new_tabs_with_layout(&layout);

                    if self.configuration.sync {
                        toggle_active_tab_sync();
                    }
                }
                _ => {}
            }
        }

        should_render
    }
    fn pipe(&mut self, _pipe_message: PipeMessage) -> bool {
        let should_render = false;
        // react to data piped to this plugin from the CLI, a keybinding or another plugin
        // read more about pipes: https://zellij.dev/documentation/plugin-pipes
        // return true if this plugin's `render` function should be called for the plugin to render
        // itself
        should_render
    }
    fn render(&mut self, _rows: usize, _cols: usize) {
        print_nested_list(self.get_nested_list(self.current_selected_list_index));
    }
}

impl State {
    fn get_nested_list(&self, selected_index: usize) -> Vec<NestedListItem> {
        let mut current_index: usize = 0;
        let mut items = vec![];

        for group in self.server_groups.iter() {
            let mut group_item = NestedListItem::new(group.get_name());
            let mut servers_items = vec![];

            if current_index == selected_index {
                group_item = group_item.selected();
            }

            current_index += 1;

            for server in group.get_servers().iter() {
                let mut server_item = NestedListItem::new(server.get_name()).indent(1);
                if current_index == selected_index {
                    server_item = server_item.selected();
                }
                if self.selected_servers.contains(server) {
                    server_item = server_item.success_color_all();
                }

                servers_items.push(server_item);
                current_index += 1;
            }

            if group.all_selected_in(&self.selected_servers) {
                group_item = group_item.success_color_all();
            }
            items.push(group_item);
            items.append(&mut servers_items);
        }

        items
    }
    fn get_displayed_lines_number(&self) -> usize {
        // self.server_groups.len()
        self.server_groups
            .iter()
            .map(|group| group.get_servers().len() + 1)
            .sum()
    }
    fn resolve_index(&self, index: usize) -> Option<(usize, Option<usize>)> {
        let mut current = 0;
        for (g_idx, group) in self.server_groups.iter().enumerate() {
            if current == index {
                return Some((g_idx, None)); // the group itself
            }
            current += 1;
            for (s_idx, _server) in group.get_servers().iter().enumerate() {
                if current == index {
                    return Some((g_idx, Some(s_idx)));
                }
                current += 1;
            }
        }
        None
    }
}

impl Configuration {
    fn from_config(config: BTreeMap<String, String>) -> Self {
        Configuration {
            layout: ZellijLayout::from_str(config.get("layout").unwrap_or(&"default".to_string())),
            sync: config
                .get("sync")
                .unwrap_or(&"false".to_string())
                .parse()
                .unwrap(),
            config_path: config
                .get("config_path")
                .map(PathBuf::from)
                .unwrap_or(Configuration::get_default_config_path()),
        }
    }

    fn get_default_config_path() -> PathBuf {
        let base = std::env::var("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                PathBuf::from(std::env::var("HOME").unwrap_or_default()).join(".config")
            });

        base.join("zellij-cssh").join("servers.toml")
    }
}

impl ZellijLayout {
    fn from_str(layout: &str) -> Self {
        match layout {
            "default" => Self::Default,
            "compact" => Self::Compact,
            _ => Self::Default,
        }
    }

    fn get_kdl(&self, tab_name: &str, panes: &str) -> String {
        match self {
            Self::Default => {
                format!(
                    r#"layout {{
                        pane_template name="ssh" command="ssh" close_on_exit=true
                        tab name="{tab_name}" {{
                            pane size=1 borderless=true {{
                                plugin location="tab-bar"
                            }}
                            {panes}
                            pane size=1 borderless=true {{
                                plugin location="status-bar"
                            }}
                        }}
                    }}"#
                )
            }
            Self::Compact => {
                format!(
                    r#"layout {{
                        pane_template name="ssh" command="ssh" close_on_exit=true
                        tab name="{tab_name}" {{
                            {panes}
                            pane size=1 borderless=true {{
                                plugin location="zellij:compact-bar"
                            }}
                        }}
                    }}"#
                )
            }
        }
    }
}
