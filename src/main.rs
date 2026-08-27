#![windows_subsystem = "windows"]

use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::thread;

use anyhow::{anyhow, Result};
use rand::Rng;
use serde_json::Value;
use slint::SharedString;
use tempfile::tempdir;
use zip::ZipArchive;

slint::include_modules!();

// ---------- 混淆核心 ----------

fn random_name(len: usize) -> String {
    rand::thread_rng()
        .sample_iter(&rand::distributions::Alphanumeric)
        .take(len)
        .map(char::from)
        .collect()
}

fn load_project(sb3_path: &Path) -> Result<Value> {
    let file = File::open(sb3_path)?;
    let mut archive = ZipArchive::new(file)?;
    let mut project_json = archive.by_name("project.json")?;
    let mut content = String::new();
    project_json.read_to_string(&mut content)?;
    Ok(serde_json::from_str(&content)?)
}

fn save_project(data: &Value, sb3_in: &Path, sb3_out: &Path) -> Result<()> {
    let temp_dir = tempdir()?;
    let temp_path = temp_dir.path();

    let file = File::open(sb3_in)?;
    let mut archive = ZipArchive::new(file)?;
    archive.extract(temp_path)?;

    let json_path = temp_path.join("project.json");
    std::fs::write(&json_path, serde_json::to_string(&data)?)?;

    let out_file = File::create(sb3_out)?;
    let mut zip_out = zip::ZipWriter::new(out_file);
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);

    for entry in walkdir::WalkDir::new(temp_path)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        if path.is_file() {
            let relative = path.strip_prefix(temp_path).unwrap();
            zip_out.start_file(relative.to_str().unwrap(), options)?;
            let mut f = File::open(path)?;
            std::io::copy(&mut f, &mut zip_out)?;
        }
    }
    zip_out.finish()?;
    Ok(())
}

fn get_name_from_obj(obj: &Value) -> Option<String> {
    match obj {
        Value::Object(map) => map.get("name").and_then(|v| v.as_str()).map(String::from),
        Value::Array(arr) if !arr.is_empty() => arr[0].as_str().map(String::from),
        _ => None,
    }
}

fn set_name_in_obj(obj: &mut Value, new_name: &str) -> bool {
    match obj {
        Value::Object(map) => {
            if map.contains_key("name") {
                map.insert("name".to_string(), Value::String(new_name.to_string()));
                true
            } else {
                false
            }
        }
        Value::Array(arr) if !arr.is_empty() => {
            arr[0] = Value::String(new_name.to_string());
            true
        }
        _ => false,
    }
}

fn obfuscate(
    sb3_in: &Path,
    sb3_out: &Path,
    rename_vars: bool,
    rename_lists: bool,
    rename_sprites: bool,
    mut log_callback: impl FnMut(&str),
) -> Result<String> {
    log_callback("开始加载项目...");
    let mut data = load_project(sb3_in)?;

    let targets = data
        .get_mut("targets")
        .and_then(|v| v.as_array_mut())
        .ok_or_else(|| anyhow!("无法找到 targets 数组"))?;

    let mut var_mapping = HashMap::new();
    let mut list_mapping = HashMap::new();
    let mut sprite_mapping = HashMap::new();
    let mut all_used_names = HashSet::new();

    log_callback("收集并重命名变量、列表、角色...");

    for target in targets.iter_mut() {
        // 变量
        if rename_vars {
            if let Some(vars) = target.get_mut("variables") {
                match vars {
                    Value::Object(map) => {
                        let mut to_rename = Vec::new();
                        for (key, val) in map.iter_mut() {
                            if let Some(old) = get_name_from_obj(val) {
                                to_rename.push((key.clone(), old, val));
                            }
                        }
                        for (_, old, val) in to_rename {
                            let mut new = random_name(8);
                            while all_used_names.contains(&new) {
                                new = random_name(8);
                            }
                            all_used_names.insert(new.clone());
                            var_mapping.insert(old, new.clone());
                            set_name_in_obj(val, &new);
                        }
                    }
                    Value::Array(arr) => {
                        for val in arr.iter_mut() {
                            if let Some(old) = get_name_from_obj(val) {
                                let mut new = random_name(8);
                                while all_used_names.contains(&new) {
                                    new = random_name(8);
                                }
                                all_used_names.insert(new.clone());
                                var_mapping.insert(old, new.clone());
                                set_name_in_obj(val, &new);
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        // 列表
        if rename_lists {
            if let Some(lists) = target.get_mut("lists") {
                match lists {
                    Value::Object(map) => {
                        let mut to_rename = Vec::new();
                        for (key, val) in map.iter_mut() {
                            if let Some(old) = get_name_from_obj(val) {
                                to_rename.push((key.clone(), old, val));
                            }
                        }
                        for (_, old, val) in to_rename {
                            let mut new = random_name(8);
                            while all_used_names.contains(&new) {
                                new = random_name(8);
                            }
                            all_used_names.insert(new.clone());
                            list_mapping.insert(old, new.clone());
                            set_name_in_obj(val, &new);
                        }
                    }
                    Value::Array(arr) => {
                        for val in arr.iter_mut() {
                            if let Some(old) = get_name_from_obj(val) {
                                let mut new = random_name(8);
                                while all_used_names.contains(&new) {
                                    new = random_name(8);
                                }
                                all_used_names.insert(new.clone());
                                list_mapping.insert(old, new.clone());
                                set_name_in_obj(val, &new);
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        // 角色名
        if rename_sprites {
            if let Some(is_stage) = target.get("isStage").and_then(|v| v.as_bool()) {
                if !is_stage {
                    if let Some(name) = target.get_mut("name").and_then(|v| v.as_str()) {
                        let old = name.to_string();
                        let mut new = random_name(8);
                        while all_used_names.contains(&new) {
                            new = random_name(8);
                        }
                        all_used_names.insert(new.clone());
                        sprite_mapping.insert(old, new.clone());
                        *target.get_mut("name").unwrap() = Value::String(new);
                    }
                }
            }
        }
    }

    log_callback("替换积木中的引用...");

    const SPRITE_FIELDS: [&str; 5] = ["SPRITE", "TO", "CLONE_OPTION", "OBJECT", "TOUCHINGOBJECTMENU"];
    const EXCLUDED_FIELDS: [&str; 4] = ["EFFECT", "VALUE", "CURRENTMENU", "NUMBER"];

    for target in targets.iter_mut() {
        if let Some(blocks) = target.get_mut("blocks").and_then(|v| v.as_object_mut()) {
            for block in blocks.values_mut() {
                if let Some(fields) = block.get_mut("fields").and_then(|v| v.as_object_mut()) {
                    if rename_vars {
                        if let Some(var_field) = fields.get_mut("VARIABLE") {
                            if let Some(arr) = var_field.as_array_mut() {
                                if let Some(first) = arr.get_mut(0) {
                                    if let Some(old) = first.as_str() {
                                        if let Some(new) = var_mapping.get(old) {
                                            *first = Value::String(new.clone());
                                        }
                                    }
                                }
                            }
                        }
                    }
                    if rename_lists {
                        if let Some(list_field) = fields.get_mut("LIST") {
                            if let Some(arr) = list_field.as_array_mut() {
                                if let Some(first) = arr.get_mut(0) {
                                    if let Some(old) = first.as_str() {
                                        if let Some(new) = list_mapping.get(old) {
                                            *first = Value::String(new.clone());
                                        }
                                    }
                                }
                            }
                        }
                    }
                    if rename_sprites {
                        for field_name in SPRITE_FIELDS.iter() {
                            if EXCLUDED_FIELDS.contains(field_name) {
                                continue;
                            }
                            if let Some(field) = fields.get_mut(*field_name) {
                                if let Some(arr) = field.as_array_mut() {
                                    if let Some(first) = arr.get_mut(0) {
                                        if let Some(old) = first.as_str() {
                                            if let Some(new) = sprite_mapping.get(old) {
                                                *first = Value::String(new.clone());
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    log_callback("保存混淆后的项目...");
    save_project(&data, sb3_in, sb3_out)?;

    let stats = format!(
        "混淆完成！\n  变量重命名: {} 个\n  列表重命名: {} 个\n  角色重命名: {} 个\n输出文件: {}",
        var_mapping.len(),
        list_mapping.len(),
        sprite_mapping.len(),
        sb3_out.display()
    );
    log_callback(&stats);
    Ok(stats)
}

// ---------- GUI 启动 ----------

fn main() -> Result<()> {
    let app = MainWindow::new()?;
    let window_weak = app.as_weak();

    // 浏览输入文件
    app.on_browse_input({
        let window_weak = window_weak.clone();
        move || {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("Scratch 项目", &["sb3"])
                .pick_file()
            {
                let path_str = path.display().to_string();
                let window = window_weak.unwrap();
                window.set_input_path(SharedString::from(&path_str));
                // 自动填充输出
                if window.get_output_path().is_empty() {
                    if let Some(stem) = path.file_stem() {
                        let base = stem.to_string_lossy();
                        window.set_output_path(SharedString::from(format!("{}-obf-manual.sb3", base)));
                    }
                }
            }
        }
    });

    // 浏览输出文件
    app.on_browse_output({
        let window_weak = window_weak.clone();
        move || {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("Scratch 项目", &["sb3"])
                .save_file()
            {
                let path_str = path.display().to_string();
                window_weak.unwrap().set_output_path(SharedString::from(&path_str));
            }
        }
    });

    // 执行混淆
    app.on_run_obfuscation({
        let window_weak = window_weak.clone();
        move || {
            let window = window_weak.unwrap();
            if window.get_running() {
                return;
            }

            let input = window.get_input_path().to_string();
            let output = window.get_output_path().to_string();
            let rename_vars = window.get_rename_vars();
            let rename_lists = window.get_rename_lists();
            let rename_sprites = window.get_rename_sprites();

            if input.is_empty() || output.is_empty() {
                let msg = "错误：请选择输入和输出文件".to_string();
                let window2 = window_weak.clone();
                slint::invoke_from_event_loop(move || {
                    let w = window2.unwrap();
                    let log = w.get_log_text().to_string();
                    w.set_log_text(SharedString::from(format!("{}{}\n", log, msg)));
                });
                return;
            }
            let input_path = PathBuf::from(&input);
            let output_path = PathBuf::from(&output);
            if !input_path.exists() {
                let msg = "错误：输入文件不存在".to_string();
                let window2 = window_weak.clone();
                slint::invoke_from_event_loop(move || {
                    let w = window2.unwrap();
                    let log = w.get_log_text().to_string();
                    w.set_log_text(SharedString::from(format!("{}{}\n", log, msg)));
                });
                return;
            }
            if input == output {
                let msg = "错误：输入和输出文件不能相同".to_string();
                let window2 = window_weak.clone();
                slint::invoke_from_event_loop(move || {
                    let w = window2.unwrap();
                    let log = w.get_log_text().to_string();
                    w.set_log_text(SharedString::from(format!("{}{}\n", log, msg)));
                });
                return;
            }

            // 设置运行状态
            slint::invoke_from_event_loop({
                let window = window_weak.clone();
                move || {
                    let w = window.unwrap();
                    w.set_running(true);
                    w.set_log_text(SharedString::from("")); // 清空日志
                }
            });

            // 启动线程
            let window_weak_for_thread = window_weak.clone();
            thread::spawn(move || {
                // 克隆一份给日志回调
                let window_for_log = window_weak_for_thread.clone();
                let log_callback = move |msg: &str| {
                    let msg = msg.to_string();
                    let window = window_for_log.clone();
                    slint::invoke_from_event_loop(move || {
                        let w = window.unwrap();
                        let log = w.get_log_text().to_string();
                        let new_log = format!("{}{}\n", log, msg);
                        w.set_log_text(SharedString::from(&new_log));
                    });
                };

                let result = obfuscate(
                    &input_path,
                    &output_path,
                    rename_vars,
                    rename_lists,
                    rename_sprites,
                    log_callback,
                );

                // 克隆另一份用于最终状态更新
                let window_for_final = window_weak_for_thread.clone();
                slint::invoke_from_event_loop(move || {
                    let w = window_for_final.unwrap();
                    w.set_running(false);
                    if let Err(e) = result {
                        let log = w.get_log_text().to_string();
                        let err_msg = format!("发生错误: {}", e);
                        w.set_log_text(SharedString::from(format!("{}{}\n", log, err_msg)));
                    }
                });
            });
        }
    });

    app.run()?;
    Ok(())
}