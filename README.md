# MCP Git Summarizer

一个基于 Rust 实现的 Model Context Protocol (MCP) 服务，旨在通过 AI 自动化管理 Git 提交流程。它可以帮助你列出未暂存文件、选择性暂存、自动生成符合规范的提交信息并执行提交。

## 核心特性

- **暂存管理**：列出未暂存/未跟踪的文件，并支持选择性暂存（`git add`）。
- **AI 差异分析**：自动获取暂存区的 `diff` 内容，为生成提交信息提供上下文。
- **结构化提交信息**：生成符合预设格式的提交信息，包含中英文描述、Log、PMS 单号及影响范围。
- **强制交互流程**：在执行提交前必须经过用户预览和确认，确保提交质量。
- **初始提交支持**：完美支持在没有任何提交记录的新仓库中执行初始提交（Initial Commit）。
- **PMS 灵活性**：主动询问 PMS 单号（如 BUG-123/TASK-456），并支持在无单号时自动清理格式。

## 安装与构建

确保你的系统已安装 [Rust](https://www.rust-lang.org/) 环境。

```bash
# 克隆项目
git clone <your-repo-url>
cd mcp-git-summarizer

# 编译 release 版本
cargo build --release
```

编译后的二进制文件位于 `target/release/mcp-git-summarizer`。

## 配置 MCP 客户端

将以下配置添加到你的 MCP 客户端（如 Claude Desktop 或 Neovim 的 MCP 插件设置）中：

```json
{
  "mcpServers": {
    "git-summarizer": {
      "command": "/path/to/mcp-git-summarizer/target/release/mcp-git-summarizer",
      "options": {
        "commitFormat": [
          "<type>[optional scope]: <english description>",
          "",
          "[English body]",
          "",
          "[Chinese body]",
          "",
          "Log: [short description of the change use chinese language]",
          "PMS: <BUG-number> or <TASK-number> (必须包含 'BUG-' 或 'TASK-' 前缀。如果没有，必须询问用户；若用户明确不提供，则从提交信息中删除此行)",
          "Influence: Explain in Chinese the potential impact of this submission."
        ],
        "extraConstraints": [
          "Body 的每一行不得超过 80 个字符。",
          "中英文 Body 必须成对出现，不得只写其中一个。"
        ]
      }
    }
  }
}
```

### 配置项说明 (Options)

| 配置项             | 说明                                                         | 默认值                                                       |
| :----------------- | :----------------------------------------------------------- | :----------------------------------------------------------- |
| `commitFormat`     | 定义 AI 生成提交信息的模板。支持占位符。                     | 预设的结构化提交模板（包含 Type, Body, Log, PMS, Influence） |
| `extraConstraints` | 字符串数组。定义 AI 在生成提交信息时必须遵守的额外约束条件。 | 限制行宽 80 字符及中英文 Body 成对出现                       |

## 可用工具 (Tools)

- `list_unstaged`: 列出所有未暂存或未跟踪的文件。
- `stage_files`: 将指定文件路径添加到暂存区。
- `get_staged_diff`: 获取暂存区差异并生成提交信息草稿。
- `execute_commit`: 执行最终的提交操作。

## 开发

本项目基于 `git2-rs` 库直接与 Git 底层交互，不依赖系统安装的 `git` 命令行工具（但在配置用户身份时仍需 `git config`）。

## 许可证

MIT
