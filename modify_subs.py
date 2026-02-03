import re
import sys

file_path = "crates/binary_options_tools/src/pocketoption/modules/subscriptions.rs"

with open(file_path, "r") as f:
    content = f.read()

# Replace match sub_type with period_secs
# Pattern matches:
# let period = match sub_type {
#     SubscriptionType::TimeAligned { duration, .. } => duration.as_secs() as u32,
#     _ => 1,
# };
# It handles whitespace variations.

pattern_period = re.compile(r'let period = match sub_type \{\s+SubscriptionType::TimeAligned \{ duration, \.\. \} => duration\.as_secs\(\) as u32,\s+_ => 1,\s+\};', re.DOTALL)
pattern_period2 = re.compile(r'let period = match sub_type \{\s+SubscriptionType::TimeAligned \{ duration, \.\. \} => \{\s+duration\.as_secs\(\) as u32\s+\}\s+_ => 1,\s+\};', re.DOTALL)

# Check occurrences
matches = pattern_period.findall(content)
matches2 = pattern_period2.findall(content)
print(f"Found {len(matches)} matches for pattern 1")
print(f"Found {len(matches2)} matches for pattern 2")

new_content = pattern_period.sub('let period = sub_type.period_secs().unwrap_or(1);', content)
new_content = pattern_period2.sub('let period = sub_type.period_secs().unwrap_or(1);', new_content)

# Replace History handling
# We need to find the block handling ServerResponse::History

history_pattern = re.compile(r'Ok\(ServerResponse::History\(data\)\) => \{.+?if let Some\(command_id\) = id \{.+?let symbol = data\.asset\.clone\(\);.+?let candles = if let Some\(candles\) = data\.candles \{.+?\}\s*else if let Some\(history\) = data\.history \{.+?\}\s*else \{.+?\};\s+if let Err\(e\) = self\.command_responder\.send\(CommandResponse::History \{.+?\}\)\.await \{.+?\}\s*\}\s*\}', re.DOTALL | re.MULTILINE)

# I will construct the regex carefully or search for specific substring to replace.
# The original code segment:
original_history_block = r"""                                        let candles = if let Some(candles) = data.candles {
                                            candles.into_iter()
                                                .map(|c| Candle::try_from((c, symbol.clone())))
                                                .collect::<Result<Vec<_>, _>>()
                                                .map_err(|e| CoreError::Other(e.to_string()))?
                                        } else if let Some(history) = data.history {
                                            compile_candles_from_ticks(&history, data.period, &symbol)
                                        } else {
                                            Vec::new()
                                        };

                                        if let Err(e) = self.command_responder.send(CommandResponse::History {
                                            command_id,
                                            data: candles
                                        }).await {
                                            warn!(target: "SubscriptionsApiModule", "Failed to send history response: {}", e);
                                        }"""

new_history_block = r"""                                        let candles_res = if let Some(candles) = data.candles {
                                            candles.into_iter()
                                                .map(|c| Candle::try_from((c, symbol.clone())))
                                                .collect::<Result<Vec<_>, _>>()
                                                .map_err(|e| PocketError::General(e.to_string()))
                                        } else if let Some(history) = data.history {
                                            Ok(compile_candles_from_ticks(&history, data.period, &symbol))
                                        } else {
                                            Ok(Vec::new())
                                        };

                                        match candles_res {
                                            Ok(candles) => {
                                                if let Err(e) = self.command_responder.send(CommandResponse::History {
                                                    command_id,
                                                    data: candles
                                                }).await {
                                                    warn!(target: "SubscriptionsApiModule", "Failed to send history response: {}", e);
                                                }
                                            }
                                            Err(e) => {
                                                if let Err(e) = self.command_responder.send(CommandResponse::HistoryFailed {
                                                    command_id,
                                                    error: Box::new(e)
                                                }).await {
                                                    warn!(target: "SubscriptionsApiModule", "Failed to send history failed response: {}", e);
                                                }
                                            }
                                        }"""

# Since spacing might differ, I will try to locate by unique strings and replace.
# "map_err(|e| CoreError::Other(e.to_string()))?" is unique enough in that context?
# But it's inside the if/else block.

# I will use a simpler approach: read the file, locate the lines, and replace.
# I know the context from the  output.

# Find the start of the block
start_marker = "let candles = if let Some(candles) = data.candles {"
end_marker = 'warn!(target: "SubscriptionsApiModule", "Failed to send history response: {}", e);'

start_idx = new_content.find(start_marker)
if start_idx == -1:
    print("Could not find history block start")
    sys.exit(1)

end_idx_sub = new_content.find(end_marker, start_idx)
if end_idx_sub == -1:
    print("Could not find history block end")
    sys.exit(1)

# Find the closing brace and semi-colon for the if let Err... block
# The end marker is inside the block.
# }
# }
# So we need to find the } after the end marker, then another }.
# Actually, I can just find the range covering the old block.

# The old block ends with:
#                                         }
#                                     }
#                                 }

# Let's try to match exact string if possible.
# I'll normalize whitespace for matching? No, dangerous.

# I will simply locate the lines and replace.
lines = new_content.split('\n')
start_line = -1
end_line = -1

for i, line in enumerate(lines):
    if "let candles = if let Some(candles) = data.candles {" in line:
        start_line = i
        break

if start_line != -1:
    # Look for the end of the block
    for i in range(start_line, len(lines)):
        if 'warn!(target: "SubscriptionsApiModule", "Failed to send history response: {}", e);' in lines[i]:
            # The block ends a few lines after this (closing braces)
            if lines[i+1].strip() == "}" and lines[i+2].strip() == "}": # Approximate
                 end_line = i + 2
                 break
            # Or maybe just i+1
            if lines[i+1].strip() == "}":
                 end_line = i + 1
                 break

if start_line != -1 and end_line != -1:
    print(f"Replacing lines {start_line} to {end_line}")
    # Construct new lines
    # I'll use the new_history_block string but ensure indentation matches
    # The indentation seems to be 40 spaces.

    new_lines_str = new_history_block

    # Replace the lines
    lines[start_line:end_line+1] = new_lines_str.split('\n')

    final_content = '\n'.join(lines)
    with open(file_path, "w") as f:
        f.write(final_content)
    print("Successfully modified file")
else:
    print("Failed to locate lines for history block replacement")
    # debug
    # print(new_content[start_idx:start_idx+500])
