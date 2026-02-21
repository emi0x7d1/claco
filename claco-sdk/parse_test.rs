use regex::Regex;

#[derive(Debug, Clone, PartialEq)]
pub enum ClacoResponse {
    Text(String),
    ToolCall { name: String, args: String },
}

fn parse_responses(text: &str, prompt: &str) -> Vec<ClacoResponse> {
    let mut blocks = Vec::new();

    let prompt_marker = format!("❯ {}", prompt);
    let parts: Vec<&str> = text.split(&prompt_marker).collect();
    if parts.len() < 2 {
        return blocks;
    }

    let after_prompt = parts.last().unwrap();
    let block_splits: Vec<&str> = after_prompt.split(|c| c == '⏺' || c == '●').collect();

    for split in block_splits.iter() {
        parse_block(split.trim(), &mut blocks);
    }

    blocks
}

fn parse_block(trimmed: &str, blocks: &mut Vec<ClacoResponse>) {
    if trimmed.is_empty() {
        return;
    }

    let re_tool = Regex::new(r"^(\w+)\(([^)]*)\)").unwrap();
    if let Some(caps) = re_tool.captures(trimmed) {
        blocks.push(ClacoResponse::ToolCall {
            name: caps[1].to_string(),
            args: caps[2].to_string(),
        });

        let matched_len = caps.get(0).unwrap().end();
        let remaining = trimmed[matched_len..].trim();
        if !remaining.is_empty() {
            let mut cleaned = String::new();
            for line in remaining.lines() {
                let trimmed_line = line.trim();
                if trimmed_line.starts_with('❯')
                    || trimmed_line.starts_with('·')
                    || trimmed_line.starts_with('⎿')
                    || trimmed_line.starts_with('─')
                    || trimmed_line.contains("esc to interrupt")
                {
                    continue; // Skip UI artifacts
                }
                cleaned.push_str(line);
                cleaned.push('\n');
            }
            let cleaned = cleaned.trim();
            if !cleaned.is_empty() {
                blocks.push(ClacoResponse::Text(cleaned.to_string()));
            }
        }
    } else {
        let mut cleaned = String::new();
        for line in trimmed.lines() {
            let trimmed_line = line.trim();
            if trimmed_line.starts_with('❯')
                || trimmed_line.starts_with('·')
                || trimmed_line.starts_with('⎿')
                || trimmed_line.starts_with('─')
                || trimmed_line.contains("esc to interrupt")
            {
                continue; // Skip UI artifacts
            }
            cleaned.push_str(line);
            cleaned.push('\n');
        }
        let cleaned = cleaned.trim();
        if !cleaned.is_empty() {
            blocks.push(ClacoResponse::Text(cleaned.to_string()));
        }
    }
}

fn is_finished(text: &str, prompt: &str) -> bool {
    if text.contains("esc to interrupt") {
        return false;
    }
    let prompt_marker = format!("❯ {}", prompt);
    if let Some(idx) = text.rfind(&prompt_marker) {
        let after_prompt = &text[idx + prompt_marker.len()..];
        for line in after_prompt.lines().skip(1) {
            if line.trim().starts_with('❯') {
                return true;
            }
        }
    }
    false
}

fn main() {
    let text = "❯ Generate a 3 page story
· Waddling…
─────────────────
❯ 
─────────────────
  esc to interrupt";
    println!("finished? {}", is_finished(text, "Generate a 3 page story"));
    println!("{:#?}", parse_responses(text, "Generate a 3 page story"));
}
