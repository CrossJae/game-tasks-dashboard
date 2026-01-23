use bevy::diagnostic::{
    FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin, SystemInformationDiagnosticsPlugin,
};
use bevy::prelude::*;
use bevy::window::{WindowLevel, WindowResolution};
use bevy::winit::WinitSettings;
use bevy_egui::{EguiContexts, EguiPlugin, EguiPrimaryContextPass, egui};
use chrono::{DateTime, Duration, FixedOffset, Local, NaiveDateTime, TimeZone};
use egui::{Color32, RichText};
// use core::time::Duration;
use serde::{Deserialize, Deserializer, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};
use std::fs;
use std::io::Write;
use std::sync::Arc;

#[cfg(target_os = "macos")]
use bevy::window::CompositeAlphaMode;

// 存储应用状态的资源
#[derive(Resource)]
struct AppState {
    init_time: DateTime<Local>,
    items: Vec<Item>, // 存储所有项目
    dirty: bool,      // 标记数据是否被修改
}

impl AppState {
    fn new() -> Self {
        let init_time = Local::now();

        // 读取本地json
        let example = load_items_from_json("assets/json/items_game.json");

        // 解析每个项目的 dead_time 并计算持续时间
        let items: Vec<Item> = example
            .iter()
            .map(|item: &Item| {
                // 解析 dead_time 字符串
                let target_datetime =
                    parse_datetime_from_str(&item.dead_time).unwrap_or_else(|| Local::now()); // 如果解析失败，使用当前时间

                // 计算持续时间（相对于应用启动时间）
                let duration = target_datetime - init_time;

                // 创建新的 Item 实例，包含计算好的持续时间
                Item {
                    id: item.id,
                    name: item.name.clone(),
                    title: item.title.clone(),
                    dead_time: item.dead_time.clone(),
                    status: item.status.clone(),
                    duration,
                }
            })
            .collect();

        Self {
            init_time,
            items,
            dirty: false,
        }
    }
    // 添加刷新方法
    pub fn refresh(&mut self) {
        let init_time = Local::now();
        let example = load_items_from_json("assets/json/items_game.json");

        // 重新计算所有项目
        let items: Vec<Item> = example
            .iter()
            .map(|item: &Item| {
                let target_datetime =
                    parse_datetime_from_str(&item.dead_time).unwrap_or_else(|| Local::now());
                let duration = target_datetime - init_time;

                Item {
                    id: item.id,
                    name: item.name.clone(),
                    title: item.title.clone(),
                    dead_time: item.dead_time.clone(),
                    status: item.status.clone(),
                    duration,
                }
            })
            .collect();

        // 更新资源
        self.init_time = init_time;
        self.items = items;

        self.dirty = false;
    }
    pub fn update(&mut self, id: u32, new_status: ItemStatus) {
        if let Some(item) = self.items.iter_mut().find(|t| t.id == id) {
            item.status = new_status;
            self.dirty = true;

            save_items_to_json("assets/json/items_game.json", self);
        }
    }
}

// 项目数据结构
#[derive(Deserialize, Serialize, Clone)]
struct Item {
    id: u32,
    name: String,
    title: String,
    dead_time: String,
    status: ItemStatus,
    // duration不包含在JSON中，我们在读取后计算
    // #[serde(skip_deserializing)] // 跳过JSON反序列化
    // #[serde(
    //     serialize_with = "serialize_duration", // 自定义序列的方法
    //     deserialize_with = "deserialize_duration",
    //     skip
    // )]
    #[serde(skip)]
    duration: Duration, // 存储计算好的持续时间
}

#[derive(Resource)]
struct FilterState {
    selected_name: String,
}

impl FilterState {
    fn new() -> Self {
        Self {
            selected_name: "All".to_string(),
        }
    }
}

// 🎉 一个经典的枚举实现
// #[derive(serde::Serialize, serde::Deserialize, Clone, Copy, PartialEq, Debug)]
// #[derive(Clone, Copy, PartialEq, Debug)]
#[derive(Serialize_repr, Deserialize_repr, Clone, Copy, Debug, PartialEq)]
#[repr(u8)]
pub enum ItemStatus {
    DOING = 1,
    DONE = 2,
    EXPIRED = 3,
}

impl ItemStatus {
    pub fn to_item_status(value: usize) -> Self {
        match value {
            1 => ItemStatus::DOING,
            2 => ItemStatus::DONE,
            3 => ItemStatus::EXPIRED,
            _ => {
                error!("Invalid value in to_item_status: {}", value);
                // TODO 默认值
                ItemStatus::DOING
            }
        }
    }

    pub fn to_usize(&self) -> usize {
        *self as usize
    }
}

/**
 * tab状态
 * 这里定义了一个资源的struct
 * 这个简单的tab包含两个内容，当前tab索引和tab的名称
 */
#[derive(Resource, Default)]
struct TabState {
    current_tab: usize,
    tab_names: Vec<&'static str>,
}

impl TabState {
    fn new() -> Self {
        Self {
            current_tab: 0,
            tab_names: vec!["doing", "done", "expired"],
        }
    }
}

/**
 * 单元格的文本样式
 */
pub struct ComplexText {
    text: String,
    size: [f32; 2],
    color: Color32,
    max_chars: usize,
    align: String,
}

impl ComplexText {
    pub fn new(text: impl Into<String>, size: [f32; 2]) -> Self {
        Self {
            text: text.into(),
            size,
            color: Color32::WHITE,
            max_chars: 8,
            align: "left".to_string(),
        }
    }

    pub fn show(self, ui: &mut egui::Ui) -> egui::Response {
        let display_text = if self.text.chars().count() > self.max_chars {
            let truncated: String = self.text.chars().take(self.max_chars - 3).collect();
            format!("{}...", truncated)
        } else {
            self.text.clone()
        };

        match self.align.as_str() {
            "left" => {
                ui.spacing_mut().item_spacing.x = 0.0; // 移除水平间距
                // 在固定大小区域内创建左对齐布局
                ui.allocate_ui_with_layout(
                    self.size.into(),
                    egui::Layout::left_to_right(egui::Align::LEFT)
                        .with_cross_align(egui::Align::Center),
                    |ui| {
                        ui.label(RichText::new(display_text).color(self.color))
                        // .on_hover_text(tooltip)
                    },
                )
                .inner
            }
            _ => {
                ui.with_layout(egui::Layout::top_down(egui::Align::RIGHT), |ui| {
                    ui.add_sized(
                        self.size,
                        egui::Label::new(RichText::new(display_text).color(self.color)),
                    )
                })
                .inner
            } // _ => ui.add_sized(
              //     self.size,
              //     egui::Label::new(RichText::new(display_text).color(self.color)),
              // ),
        }
    }
}

// ——————————————————————————————————
// 主要功能
// ——————————————————————————————————
fn main() {
    App::new()
        .insert_resource(WinitSettings::desktop_app())
        // .insert_resource(WinitSettings {
        //     focused_mode: bevy::winit::UpdateMode::Continuous,
        //     unfocused_mode: bevy::winit::UpdateMode::reactive_low_power(
        //         core::time::Duration::from_millis(10),
        //     ),
        // })
        .add_plugins(
            (DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    // transparent: true,
                    // decorations: false,
                    resolution: WindowResolution::new(375, 500), // 设置窗口大小为 400x400
                    // window_level: WindowLevel::AlwaysOnTop,
                    #[cfg(target_os = "macos")]
                    composite_alpha_mode: CompositeAlphaMode::PostMultiplied,
                    present_mode: bevy::window::PresentMode::AutoVsync, // 这是关键修复
                    // Turn off vsync to maximize CPU/GPU usage
                    // cpu飙升到300了
                    // present_mode: bevy::window::PresentMode::AutoNoVsync,
                    ..default()
                }),
                ..default()
            })),
        )
        // 添加帧时间诊断 (用于计算FPS)
        // 添加系统信息诊断 (包含内存和CPU使用率)
        // 添加日志输出插件，让数据在控制台显示
        // .add_plugins((
        //     FrameTimeDiagnosticsPlugin::default(),
        //     SystemInformationDiagnosticsPlugin::default(),
        //     LogDiagnosticsPlugin::default(),
        // ))
        .add_plugins(EguiPlugin::default())
        .insert_resource(AppState::new())
        .insert_resource(FilterState::new())
        .insert_resource(TabState::new())
        .add_systems(Startup, setup_camera_system)
        .add_systems(Update, setup_chinese_font)
        .add_systems(EguiPrimaryContextPass, task_operate_system)
        .run();
}

/**
 * 必须增加的摄像机
 */
fn setup_camera_system(mut commands: Commands) {
    commands.spawn(Camera2d::default());
}

/**
 * 支持中文，设置了flag，防止每帧都更新
 */
fn setup_chinese_font(
    mut contexts: EguiContexts,
    // mut has_setup: Local<bool>, // 这是一个局部状态，每个系统实例独享
    mut has_setup: bevy::prelude::Local<bool>,
) {
    // 如果已经设置过，直接返回
    if *has_setup {
        return;
    }
    if let Ok(egui_ctx) = contexts.ctx_mut() {
        let mut fonts = egui::FontDefinitions::default();
        // 1. 创建 FontData
        let font_data = egui::FontData::from_static(include_bytes!("../assets/fonts/PingFang.ttc"));

        // 2. 使用 Arc::new 将其包装为 Arc<FontData>
        fonts.font_data.insert(
            "my_macos_font".to_owned(),
            Arc::new(font_data), // 关键修改：包装成 Arc
        );

        // 3. 设置字体族（这部分不变）
        // fonts
        //     .families
        //     .entry(egui::FontFamily::Proportional)
        //     .or_default()
        //     .insert(0, "my_macos_font".to_owned());

        // fonts
        //     .families
        //     .entry(egui::FontFamily::Monospace)
        //     .or_default()
        //     .push("my_macos_font".to_owned());

        // 尝试：完全替换比例字体族，而不是插入
        fonts.families.insert(
            egui::FontFamily::Proportional,
            vec!["my_macos_font".to_owned()],
        );
        // 等宽字体族也可以同样处理
        fonts.families.insert(
            egui::FontFamily::Monospace,
            vec!["my_macos_font".to_owned()],
        );
        println!(
            "  1. 注册的字体数据键名: {:?}",
            fonts.font_data.keys().collect::<Vec<_>>()
        );
        // println!(
        //     "  2. 比例字体系列配置: {:?}",
        //     fonts.families.get(&egui::FontFamily::Proportional)
        // );
        // println!(
        //     "  3. 等宽字体系列配置: {:?}",
        //     fonts.families.get(&egui::FontFamily::Monospace)
        // );

        // 4. 应用字体
        egui_ctx.set_fonts(fonts);

        // 调试：打印所有已注册的字体数据键名
        // println!(
        //     "当前已注册的字体键名: {:?}",
        //     egui_ctx.fonts(|f| f.families().into_iter().cloned().collect::<Vec<_>>())
        // );
        // 可选：如果你还想查看字体系列的配置，可以添加这行
        // println!(
        //     "当前字体系列配置: {:?}",
        //     egui_ctx.fonts(|f| f.definitions().families.clone())
        // );

        // 标记为已设置，下次这个系统被调用时会直接跳过
        *has_setup = true;
    }
}

/**
 * 主UI逻辑
 */
fn task_operate_system(
    mut contexts: EguiContexts,
    mut app_state: ResMut<AppState>,
    mut filter_state: ResMut<FilterState>,
    mut tabs: ResMut<TabState>,
) {
    // 处理 Result，如果出错则直接返回
    let ctx = match contexts.ctx_mut() {
        Ok(ctx) => ctx,
        Err(_) => return,
    };

    // ctx.set_pixels_per_point(1.0);

    // TEST CODE
    // egui::CentralPanel::default().show(ctx, |ui| {
    //     ui.label("hello world");
    //     ui.label(format!(
    //         "当前时间: {}",
    //         Local::now().format("%Y-%m-%d %H:%M:%S")
    //     ));
    // });

    // NOUSE
    // setup_chinese_font(ctx);

    // 也不是非要使用window
    // egui::Window::new("TestWindow").show(ctx, |ui| {
    // });

    // tab
    // let ctx = contexts.ctx_mut().expect("REASON");
    let mut clicked_tab = None;

    egui::TopBottomPanel::top("tabs").show(ctx, |ui| {
        ui.horizontal(|ui| {
            for (i, &name) in tabs.tab_names.iter().enumerate() {
                let selected = tabs.current_tab == i;
                if ui.selectable_label(selected, name).clicked() {
                    clicked_tab = Some(i);
                }
            }
        });
    });

    // 在UI作用域外修改资源
    if let Some(tab_index) = clicked_tab {
        tabs.current_tab = tab_index;
    }

    egui::CentralPanel::default().show(ctx, |ui| {
        // ui.label("hello world");
        change_tab_content(ui, app_state, filter_state, tabs.current_tab);
    });
}

/**
 * 切换tab时content的表格会改变内容
 */
fn change_tab_content(
    ui: &mut egui::Ui,
    mut app_state: ResMut<AppState>,
    mut filter_state: ResMut<FilterState>,
    current_tab: usize,
) {
    // ui.label(current_tab.to_string());

    // 增加排序
    let mut sorted_items = app_state.items.clone();
    // sorted_items.sort_by_key(|item| item.duration);
    sorted_items.sort_by(|a, b| a.duration.cmp(&b.duration));

    let mut available_names: Vec<&str> =
        sorted_items.iter().map(|item| item.name.as_str()).collect();

    available_names.sort();
    available_names.dedup();

    let mut filter_options = vec!["All"];
    filter_options.extend(available_names);

    egui::ComboBox::from_id_salt("name_filter")
        .selected_text(&filter_state.selected_name)
        .show_ui(ui, |ui| {
            for &name in &filter_options {
                ui.selectable_value(&mut filter_state.selected_name, name.to_string(), name);
            }
        });

    // 表头
    ui.horizontal(|ui| {
        add_item(ui, &String::from("游戏名称"));
        add_item(ui, &String::from("活动名称"));
        add_item(ui, &String::from("剩余时间"));
        add_item(ui, &String::from("操作"));
    });
    // 循环内容
    // for item in sorted_items.iter() {
    for (index, item) in sorted_items.iter().enumerate() {
        let dur = format_duration(item.duration);
        let mut new_state = item.status;
        if dur == "expired" && item.status == ItemStatus::DOING {
            new_state = ItemStatus::EXPIRED;
        }

        // println!("new_state: {}", ItemStatus::to_usize(&new_state));

        if (filter_state.selected_name == "All" || item.name == filter_state.selected_name)
        // && new_state == current_tab + 1
        && ItemStatus::to_usize(&new_state) == current_tab + 1
        {
            // 使用 Frame 创建带背景色的水平布局区域
            let bg_color = if index % 2 == 0 {
                egui::Color32::from_hex("#333333").unwrap()
            } else {
                egui::Color32::TRANSPARENT
            };

            // 直接向 ui 添加 Frame
            egui::Frame::default()
                // .fill(bg_color)
                // .inner_margin(egui::Margin::symmetric(8.0, 4.0))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        add_item(ui, &item.name);
                        add_item(ui, &item.title);

                        match new_state {
                            ItemStatus::DOING => {
                                add_item(ui, &dur);
                                if ui.button("完成").clicked() {
                                    // 1.获取id把对应的status改成对应的值
                                    // 2.刷新列表
                                    app_state.update(item.id, ItemStatus::DONE)
                                }
                            }

                            ItemStatus::DONE => {
                                add_item(ui, &item.dead_time);
                            }

                            ItemStatus::EXPIRED => {
                                add_item(ui, &item.dead_time);
                                if ui.button("完成").clicked() {
                                    // 1.获取id把对应的status改成对应的值
                                    // 2.刷新列表
                                    app_state.update(item.id, ItemStatus::DONE)
                                }
                            }
                        }
                    });
                });
        }
    }
    // 常规部分不知道放在哪里
    // ui.separator();
    // ui.label(format!(
    //     "初始化时间: {}",
    //     app_state.init_time.format("%Y-%m-%d %H:%M:%S")
    // ));

    // NOUSE 不是实时刷新
    // ui.label(format!(
    //     "当前时间: {}",
    //     Local::now().format("%Y-%m-%d %H:%M:%S")
    // ));

    if ui.button("刷新数据").clicked() {
        // println!("点击了按钮");
        app_state.refresh();
    }
}

fn add_item(ui: &mut egui::Ui, text: &String) {
    // ui.add_sized([120.0, 20.0], egui::Label::new(text));
    ComplexText::new(text, [120.0, 20.0]).show(ui);
}

// ——————————————————————————————————
// 工具函数
// ——————————————————————————————————
/**
 * 时间处理
 */
// 解析日期时间字符串
fn parse_datetime_from_str(datetime_str: &str) -> Option<DateTime<Local>> {
    // 尝试解析带时区的格式 "YYYY-MM-DD HH:MM:SS +HH:MM"
    if let Ok(dt) = DateTime::parse_from_str(datetime_str, "%Y-%m-%d %H:%M:%S %z") {
        return Some(dt.with_timezone(&Local));
    }

    // 尝试解析不带时区的格式 "YYYY-MM-DD HH:MM:SS"
    if let Ok(naive_dt) = NaiveDateTime::parse_from_str(datetime_str, "%Y-%m-%d %H:%M:%S") {
        return Some(Local.from_local_datetime(&naive_dt).unwrap());
    }

    // 尝试解析只包含日期的格式 "YYYY-MM-DD"
    if let Ok(naive_date) = chrono::NaiveDate::parse_from_str(datetime_str, "%Y-%m-%d") {
        let naive_dt = naive_date.and_hms_opt(0, 0, 0).unwrap();
        return Some(Local.from_local_datetime(&naive_dt).unwrap());
    }

    None
}

// 格式化剩余时间
fn format_duration(duration: Duration) -> String {
    let total_seconds = duration.num_seconds();

    if total_seconds <= 0 {
        return "expired".to_string();
    }

    let days = total_seconds / (24 * 3600);
    let hours = (total_seconds % (24 * 3600)) / 3600;
    let minutes = (total_seconds % 3600) / 60;

    let mut parts = Vec::new();

    // 如果day>0 显示99d24h格式
    // 如果day<=0 显示24h60m格式
    if days > 0 {
        parts.push(format!("{}d", days));
        if hours > 0 {
            parts.push(format!("{}h", hours));
        }
    } else {
        if hours > 0 {
            parts.push(format!("{}h", hours));
        }
        if minutes > 0 {
            parts.push(format!("{}m", minutes));
        }
    }

    // 否则小于1分钟
    if parts.is_empty() {
        "<1m".to_string()
    } else {
        parts.join("")
    }
}

/**
 * json处理
 */
fn save_items_to_json(
    file_path: &str,
    app_state: &mut AppState,
) -> Result<(), Box<dyn std::error::Error>> {
    let json = serde_json::to_string_pretty(&app_state.items)?;

    let file_path_cache = std::path::Path::new(file_path);

    let temp_path = file_path_cache.with_extension("tmp");
    let mut file = std::fs::File::create(&temp_path)?;
    file.write_all(json.as_bytes())?;
    file.sync_all()?;

    fs::rename(temp_path, file_path)?;

    // info!("Saved {} tasks to {:?}", storage.tasks.len(), storage.file_path);

    Ok(())
}

fn load_items_from_json(file_path: &str) -> Vec<Item> {
    // 读取文件，失败时返回空vec
    let json_content = match fs::read_to_string(file_path) {
        Ok(content) => content,
        Err(e) => {
            eprintln!("Failed to read file {}: {}", file_path, e);
            return Vec::new();
        }
    };

    // 解析JSON，失败时返回空vec
    let mut items: Vec<Item> = match serde_json::from_str(&json_content) {
        Ok(items) => items,
        Err(e) => {
            eprintln!("Failed to parse JSON: {}", e);
            return Vec::new();
        }
    };

    // 如果是空数组，也返回空vec，但可以记录日志
    if items.is_empty() {
        eprintln!("JSON file contains no items");
        return Vec::new();
    }

    // 处理每个item的duration
    for item in &mut items {
        item.duration = Duration::zero();
    }

    items
}

/**
 * 废弃物料
 */
fn value_in_status(status: &ItemStatus) -> &str {
    match status {
        ItemStatus::DOING => "doing",
        ItemStatus::DONE => "done",
        ItemStatus::EXPIRED => "expired",
    }
}
/**
 * 显示高亮的倒计时
 */
fn highlight_duration(duration: Duration, highlight_days: i64) -> bool {
    let total_seconds = duration.num_seconds();

    if total_seconds <= 0 {
        return false;
    }

    let days = total_seconds / (24 * 3600);
    if days < highlight_days {
        return true;
    }

    return false;
}
