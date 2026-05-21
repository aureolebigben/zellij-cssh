use zellij_tile::prelude::*;

use std::collections::{BTreeMap, HashSet};

#[derive(Default)]
struct State {
    // the state of the plugin
    server_groups: Vec<ServerGroup>,
    current_selected_list_index: usize,
    selected_servers: HashSet<ServerConfig>,
}

#[derive(Clone)]
struct ServerGroup {
    name: String,
    servers: Vec<ServerConfig>,
}

#[derive(Clone, Hash, PartialEq, Eq)]
struct ServerConfig {
    name: String,
    host: String,
}

struct Configuration {

}

register_plugin!(State);

// NOTE: you can start a development environment inside Zellij by running `zellij -l zellij.kdl` in
// this plugin's folder
//
// More info on plugins: https://zellij.dev/documentation/plugins

impl ZellijPlugin for State {
    fn load(&mut self, configuration: BTreeMap<String, String>) {

        request_permission(&[
            PermissionType::ChangeApplicationState,
        ]);

        // TODO Remove test data

        self.server_groups = vec![
            ServerGroup {
                name: "dbs".to_string(),
                servers: vec![
                    ServerConfig {
                        name: "db-01".to_string(),
                        host: "k-db-01".to_string(),
                    },
                    ServerConfig {
                        name: "db-02".to_string(),
                        host: "k-db-02".to_string(),
                    },
                ],
            },
            ServerGroup {
                name: "deb11".to_string(),
                servers: vec![
                    ServerConfig {
                        name: "deb11-01".to_string(),
                        host: "deb11-prod01".to_string(),
                    },
                    ServerConfig {
                        name: "deb11-02".to_string(),
                        host: "deb11-prod02".to_string(),
                    },
                ],
            },
        ];

        subscribe(&[EventType::Key]);
        // runs once on plugin load, provides the configuration with which this plugin was loaded
        // (if any)
        //
        // this is a good place to `subscribe` (https://docs.rs/zellij-tile/latest/zellij_tile/shim/fn.subscribe.html)
        // to `Event`s (https://docs.rs/zellij-tile/latest/zellij_tile/prelude/enum.Event.html)
        // and `request_permissions` (https://docs.rs/zellij-tile/latest/zellij_tile/shim/fn.request_permission.html)
    }
    fn update(&mut self, event: Event) -> bool {
        let mut should_render = false;

        match event {
            Event::Key(key) => match key.bare_key {
                BareKey::Down => {
                    if self.current_selected_list_index < self.get_displayed_lines_number() - 1 {
                        self.current_selected_list_index += 1;
                        should_render = true
                    }
                }
                BareKey::Up => {
                    if self.current_selected_list_index > 0 {
                        self.current_selected_list_index -= 1;
                        should_render = true
                    }
                }
                BareKey::Char(c) => {
                    if c == ' ' {
                        if let Some((group_idx, server_idx)) = self.resolve_index(self.current_selected_list_index) {
                            match server_idx {
                                Some(s_idx) => {
                                    let server = &self.server_groups[group_idx].servers[s_idx];
                                    if !self.selected_servers.contains(server) {
                                        self.selected_servers.insert(server.clone());
                                    } else {
                                        self.selected_servers.remove(server);
                                    }
                                }
                                None => {
                                    let group = &self.server_groups[group_idx];

                                    if group.all_selected_in(&self.selected_servers) {
                                        for server in &group.servers {
                                            self.selected_servers.remove(server);
                                        }
                                    } else {
                                        for server in &self.server_groups[group_idx].servers {
                                            self.selected_servers.insert(server.clone());
                                        }
                                    }
                                }
                            }
                            should_render = true;
                        }
                    }
                }
                BareKey::Enter => {
                    let tab_name = "cssh";
                    let panes = self.selected_servers.iter().map(|server| {
                        format!(r#"
                        ssh {{
                            args "{host}"
                        }}
                        "#, host = server.host)
                    }).collect::<Vec<String>>().join("\n");

                    let layout = format!(r#"
                    layout {{
                        pane_template name="ssh" command="ssh" close_on_exit=true
                        tab name="{tab_name}" {{
                            {panes}
                            pane size=1 borderless=true {{
                                plugin location="zellij:compact-bar"
                            }}
                        }}
                    }}
                    "#);
                    eprintln!("DEBUG layout:\n{}", layout);
                    new_tabs_with_layout(&layout);
                    toggle_active_tab_sync();
                }
                _ => {}
            },
            _ => {}
        }

        should_render
    }
    fn pipe(&mut self, pipe_message: PipeMessage) -> bool {
        let mut should_render = false;
        // react to data piped to this plugin from the CLI, a keybinding or another plugin
        // read more about pipes: https://zellij.dev/documentation/plugin-pipes
        // return true if this plugin's `render` function should be called for the plugin to render
        // itself
        should_render
    }
    fn render(&mut self, rows: usize, cols: usize) {
        print_nested_list(self.get_nested_list(self.current_selected_list_index));
    }
}

impl State {
    fn get_nested_list(&self, selected_index: usize) -> Vec<NestedListItem> {
        let mut current_index: usize = 0;
        let mut items = vec![];

        for group in self.server_groups.iter() {
            let mut group_item = NestedListItem::new(&group.name);
            let mut servers_items = vec![];

            if current_index == selected_index {
                group_item = group_item.selected();
            }

            current_index += 1;

            for server in group.servers.iter() {
                let mut server_item = NestedListItem::new(&server.name).indent(1);
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
        self.server_groups.iter().map(|group| {
            group.servers.len() + 1
        }).sum()
    }
    fn resolve_index(&self, index: usize) -> Option<(usize, Option<usize>)> {
        let mut current = 0;
        for (g_idx, group) in self.server_groups.iter().enumerate() {
            if current == index {
                return Some((g_idx, None)); // the group itself
            }
            current += 1;
            for (s_idx, _server) in group.servers.iter().enumerate() {
                if current == index {
                    return Some((g_idx, Some(s_idx)));
                }
                current += 1;
            }
        }
        None
    }
}

impl ServerGroup {
    fn all_selected_in(&self, servers: &HashSet<ServerConfig>) -> bool {
        !self.servers.is_empty() && self.servers.iter().all(|server| servers.contains(server))
    }
}
