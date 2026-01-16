mod git;
mod protocol;

use anyhow::Result;
use git::GitHandler;
use protocol::{CallToolParams, InitializeParams, JsonRpcRequest, Tool};

use serde_json::json;
use std::io::{self, BufRead, Write};

use std::sync::Mutex;

struct ServerConfig {
    commit_format: Vec<String>,
    extra_constraints: Vec<String>,
}

lazy_static::lazy_static! {
    static ref CONFIG: Mutex<ServerConfig> = Mutex::new(ServerConfig {
        commit_format: vec![
            "<type>[optional scope]: <english description>".to_string(),
            "".to_string(),
            "[English body]".to_string(),
            "".to_string(),
            "[Chinese body]".to_string(),
            "".to_string(),
            "Log: [short description of the change use chinese language]".to_string(),
            "PMS: <BUG-number> or <TASK-number> (必须包含 'BUG-' 或 'TASK-' 前缀。如果没有，必须询问用户；若用户明确不提供，则从提交信息中删除此行)".to_string(),
            "Influence: Explain in Chinese the potential impact of this submission.".to_string(),
        ],
        extra_constraints: vec![
            "Body 的每一行不得超过 80 个字符。".to_string(),
            "中英文 Body 必须成对出现，不得只写其中一个。".to_string(),
        ],
    });
}


#[tokio::main]
async fn main() -> Result<()> {
    let stdin = io::stdin();
    let mut lines = stdin.lock().lines();

    while let Some(Ok(line)) = lines.next() {
        eprintln!("收到请求: {}", line);
        let request: JsonRpcRequest = match serde_json::from_str(&line) {
            Ok(req) => req,
            Err(e) => {
                eprintln!("JSON 解析失败: {}", e);
                continue;
            }
        };

        let is_notification = request.id.is_none();

        let response_payload = match request.method.as_str() {
            "initialize" => {
                if let Some(params_val) = &request.params {
                    if let Ok(params) =
                        serde_json::from_value::<InitializeParams>(params_val.clone())
                    {
                        if let Some(options) = params.options {
                            let mut config = CONFIG.lock().unwrap();
                            if let Some(format_val) = options.get("commitFormat") {
                                if let Some(s) = format_val.as_str() {
                                    config.commit_format = vec![s.to_string()];
                                } else if let Some(arr) = format_val.as_array() {
                                    config.commit_format = arr
                                        .iter()
                                        .filter_map(|v| v.as_str())
                                        .map(|s| s.to_string())
                                        .collect();
                                }
                            }


                            if let Some(constraints) =
                                options.get("extraConstraints").and_then(|v| v.as_array())
                            {
                                config.extra_constraints = constraints
                                    .iter()
                                    .filter_map(|v| v.as_str())
                                    .map(|s| s.to_string())
                                    .collect();
                            }
                        }
                    }
                }

                Some(json!({
                    "protocolVersion": "2024-11-05",
                    "capabilities": {
                        "tools": {
                            "listChanged": true
                        }
                    },
                    "serverInfo": {
                        "name": "git-summarizer",
                        "version": "0.1.0"
                    }
                }))
            }
            "notifications/initialized" => {
                eprintln!("客户端已确认初始化");
                None
            }
            "tools/list" => {
                let config = CONFIG.lock().unwrap();
                let format_hint = config.commit_format.join("\n");
                let extra_constraints_hint = config

                    .extra_constraints
                    .iter()
                    .map(|c| format!("- {}", c))
                    .collect::<Vec<_>>()
                    .join("\n");

                let tools = vec![

                    Tool {
                        name: "list_unstaged".to_string(),
                        description: "列出当前项目中所有未暂存（unstaged）或未跟踪（untracked）的文件。".to_string(),
                        input_schema: json!({
                            "type": "object",
                            "properties": {}
                        }),
                    },
                    Tool {
                        name: "stage_files".to_string(),
                        description: "将指定的文件添加到 Git 暂存区。".to_string(),
                        input_schema: json!({
                            "type": "object",
                            "properties": {
                                "paths": {
                                    "type": "array",
                                    "items": { "type": "string" },
                                    "description": "要暂存的文件路径列表"
                                }
                            },
                            "required": ["paths"]
                        }),
                    },
                    Tool {
                        name: "get_staged_diff".to_string(),

                        description: format!(
                            "获取当前 git 暂存区的变更内容 (git diff --staged)。\n\n\
                            ### 工作流要求：\n\
                            1. 生成提交信息：根据变更内容总结出一个提交信息草稿。\n\
                            2. 处理 PMS 单号：\n\
                               - 如果无法确定单号，**必须**询问用户提供。\n\
                               - 如果用户提供了单号，将其填入提交信息。\n\
                               - 如果用户明确表示没有单号，**必须从最终提交信息中删除整个 PMS 行**。\n\
                            3. 用户预览与修改：展示草稿，询问用户确认。\n\
                            4. 严禁直接提交：必须得到用户明确确认后才能执行 execute_commit。\n\n\
                            ### 提交格式要求：\n{}\n\n\
                            ### 额外约束：\n{}",

                            format_hint,
                            extra_constraints_hint
                        ),

                        input_schema: json!({
                            "type": "object",
                            "properties": {}
                        }),
                    },


                    Tool {
                        name: "execute_commit".to_string(),
                        description: "执行提交。请在用户确认了你总结的提交信息后再调用此工具。".to_string(),
                        input_schema: json!({
                            "type": "object",
                            "properties": {
                                "message": { "type": "string", "description": "提交信息" }
                            },
                            "required": ["message"]
                        }),
                    },
                ];
                Some(json!({ "tools": tools }))
            }
            "tools/call" => {
                let params: CallToolParams =
                    serde_json::from_value(request.params.clone().unwrap_or_default())?;
                let tool_result = match params.name.as_str() {
                    "list_unstaged" => match GitHandler::get_unstaged_files() {
                        Ok(files) => {
                            let text = if files.is_empty() {
                                "暂无未暂存的文件。".to_string()
                            } else {
                                format!(
                                    "未暂存的文件：\n{}\n\n工作流提醒：\n1. 请向用户展示上述文件列表。\n2. **必须**请用户确认哪些文件需要被暂存（git add）。\n3. 只有在用户明确指定文件后，才可调用 `stage_files`。",
                                    files.join("\n")
                                )
                            };

                            json!({ "content": [{ "type": "text", "text": text }] })
                        }

                        Err(e) => {
                            json!({ "isError": true, "content": [{ "type": "text", "text": e.to_string() }] })
                        }
                    },
                    "stage_files" => {
                        let arguments = params.arguments.as_ref();
                        let paths = arguments
                            .and_then(|a| a["paths"].as_array())
                            .map(|arr| {
                                arr.iter()
                                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                                    .collect::<Vec<String>>()
                            })
                            .unwrap_or_default();
                        match GitHandler::stage_files(paths) {
                            Ok(_) => json!({ "content": [{ "type": "text", "text": "文件已成功暂存。\n\n提示：现在请使用 `get_staged_diff` 获取变更差异并生成提交信息草稿。" }] }),
                            Err(e) => {
                                json!({ "isError": true, "content": [{ "type": "text", "text": e.to_string() }] })
                            }
                        }

                    }
                    "get_staged_diff" => match GitHandler::get_staged_diff() {
                        Ok(diff) => {
                            let text = format!(
                                "{}\n\n工作流提醒：\n1. 请根据上述差异总结一个提交信息草稿。\n2. **必须**询问用户确认 PMS 单号（格式如 BUG-123 或 TASK-456）。\n3. 展示最终提交信息并请求用户明确确认。\n4. 只有在用户确认后，才可调用 `execute_commit`。",
                                diff
                            );
                            json!({ "content": [{ "type": "text", "text": text }] })
                        }

                        Err(e) => {
                            json!({ "isError": true, "content": [{ "type": "text", "text": e.to_string() }] })
                        }
                    },
                    "execute_commit" => {
                        let arguments = params.arguments.as_ref();
                        let msg = arguments.and_then(|a| a["message"].as_str()).unwrap_or("");
                        match GitHandler::commit(msg) {
                            Ok(res) => json!({ "content": [{ "type": "text", "text": res }] }),
                            Err(e) => {
                                json!({ "isError": true, "content": [{ "type": "text", "text": e.to_string() }] })
                            }
                        }
                    }
                    _ => {
                        json!({ "isError": true, "content": [{ "type": "text", "text": "未知工具" }] })
                    }
                };
                Some(tool_result)
            }
            _ => {
                if is_notification {
                    None
                } else {
                    Some(json!({ "error": { "code": -32601, "message": "Method not found" } }))
                }
            }
        };

        if let (Some(payload), Some(id)) = (response_payload, request.id) {
            let mut response_obj = serde_json::Map::new();
            response_obj.insert("jsonrpc".to_string(), json!("2.0"));
            response_obj.insert("id".to_string(), id);

            if let Some(error) = payload.get("error") {
                response_obj.insert("error".to_string(), error.clone());
            } else {
                response_obj.insert("result".to_string(), payload);
            }

            let output = serde_json::to_string(&response_obj)?;
            println!("{}", output);
            io::stdout().flush()?;
            eprintln!("发送响应: {}", output);
        }
    }

    Ok(())
}
