use super::*;

impl AgentCore {
    pub(super) fn inject_prompt_contract_context(
        &self,
        prompt: &str,
        literal_json_output: bool,
        tools: &[crate::llm::backend::LlmToolDecl],
        messages: &mut Vec<LlmMessage>,
    ) {
        let explicit_prompt_paths = if literal_json_output {
            Vec::new()
        } else {
            extract_explicit_file_paths(prompt)
        };
        let explicit_output_dirs = if literal_json_output {
            Vec::new()
        } else {
            extract_explicit_directory_paths(prompt)
        };

        if !explicit_prompt_paths.is_empty() {
            for prefetched_message in build_prefetched_prompt_file_messages(&explicit_prompt_paths)
            {
                if let Some(last_user_idx) =
                    messages.iter().rposition(|message| message.role == "user")
                {
                    messages.insert(last_user_idx, prefetched_message);
                } else {
                    messages.push(prefetched_message);
                }
            }
            inject_context_message(
                messages,
                format!(
                    "## File Grounding\nThe user explicitly referenced these real input files:\n{}\nInspect the relevant files before generating or executing code. Base every answer and script on the real file contents. Treat these referenced files as read-only inputs unless the user explicitly asks you to modify them. Do not invent substitute datasets, placeholder paths, or mock values.",
                    explicit_prompt_paths
                        .iter()
                        .map(|path| format!("- {}", path))
                        .collect::<Vec<_>>()
                        .join("\n")
                ),
            );
            if let Some(prefetched_context) =
                build_prefetched_prompt_file_context(&explicit_prompt_paths)
            {
                inject_context_message(messages, prefetched_context);
            }
            if let Some(problem_requirements_context) =
                build_authoritative_problem_requirements_context(&explicit_prompt_paths)
            {
                inject_context_message(messages, problem_requirements_context);
            }
        }

        if prompt_requires_persisted_level_scripts(prompt) && !explicit_output_dirs.is_empty() {
            inject_context_message(
                messages,
                format!(
                    "## Output Contract\nThe user requested persisted script files under these output directories:\n{}\nFor each run_generated_code call, generate exactly one level script. Put the exact absolute target file path in the first comment line, use the filename format 'level-N-solution.py', and do not paste fenced code or prose instead of calling run_generated_code.",
                    explicit_output_dirs
                        .iter()
                        .map(|path| format!("- {}", path))
                        .collect::<Vec<_>>()
                        .join("\n")
                ),
            );
        }

        if tools
            .iter()
            .any(|tool| tool.name == "extract_document_text")
            && prompt_requests_document_extraction(prompt)
        {
            inject_context_message(
                messages,
                "## Direct Tool Hint\nThis request already references a real document in the workspace. Call `extract_document_text` directly before using discovery tools, then write the grounded answer file from the extracted contents.".to_string(),
            );
        }
        if tools.iter().any(|tool| tool.name == "inspect_tabular_data")
            && prompt_requests_tabular_inspection(prompt)
        {
            inject_context_message(
                messages,
                "## Direct Tool Hint\nThis request references real CSV or spreadsheet files in the workspace. Call `inspect_tabular_data` directly for each relevant `.csv` or `.xlsx` input before drafting the output, instead of relying on `search_tools`, raw file reads, or file metadata alone. Use the returned `numeric_summaries`, `grouped_summaries`, row previews, and full-row payloads when available so totals, top categories, and comparisons come from computed tool output rather than mental arithmetic.".to_string(),
            );
        }
        if tools.iter().any(|tool| tool.name == "generate_image")
            && prompt_requests_image_generation(prompt)
        {
            inject_context_message(
                messages,
                "## Direct Tool Hint\nThis request is asking for a real generated image file. Call `generate_image` directly with a concrete prompt and the requested output path instead of searching for tool documentation or writing placeholder image files.".to_string(),
            );
        }
        if tools.iter().any(|tool| tool.name == "web_search")
            && prompt_requests_current_web_research(prompt)
        {
            inject_context_message(
                messages,
                "## Direct Tool Hint\nThis request depends on current external information. Call `web_search` directly with focused queries before drafting the answer, instead of starting with generic tool discovery.".to_string(),
            );
            inject_context_message(
                messages,
                "## Research Verification Hint\nWhen the task asks for current dates, URLs, or other factual fields, do not finalize from vague search snippets alone. If the official search results already expose the exact field values, synthesize directly from those grounded results. Only fetch or read cited source pages when the search results are still missing the required exact facts.".to_string(),
            );
            inject_context_message(
                messages,
                "## Multi-Item Research Guard\nFor multi-item current-research roundups, use official search results for candidate discovery and prefer direct synthesis once those results already show the exact dates, locations, and URLs you need. Download or read cited official pages only for candidates whose required fields are still missing. Save only the requested artifact and omit trailing process notes about how the research was verified unless the user asked for them.".to_string(),
            );
            if prompt_requests_conference_roundup(prompt) {
                inject_context_message(
                    messages,
                    "## Conference Quality Guard\nFor general conference roundups, prefer established annual conference series with official event pages, exact dates, and clear locations. Avoid meetup-style events, workshops, training programs, or weak local city editions unless the user explicitly asked for that narrower scope. If the official page for the current upcoming edition still does not publish an exact date or location, leave that event out and pick a different conference instead of inferring the missing details.".to_string(),
                );
                inject_context_message(
                    messages,
                    "## Conference Search Strategy\nStart with broad, organizer-diverse searches for flagship annual tech conferences that are upcoming now. If the user did not specify a year, your initial conference search query must not contain an explicit four-digit year token such as 2026 or 2027. Use neutral \"upcoming\", \"this year\", or \"official event\" wording first, then keep only entries whose official evidence clearly shows the relevant upcoming edition. Do not anchor on a single month or a narrow niche unless the user explicitly asked for that scope. Prefer globally recognized vendor, developer, cloud, infrastructure, AI, security, or cross-industry technology conferences over secondary community or product-marketing events.".to_string(),
                );
            }
            if expected_file_management_targets(prompt)
                .iter()
                .flatten()
                .any(|path| path.ends_with(".md"))
            {
                inject_context_message(
                    messages,
                    "## Research Output Contract\nIf the requested research artifact is a `.md` file, save the final result as real Markdown, not raw JSON or CSV. Prefer a short heading and then either a Markdown table or a bullet list with one item per verified entry and the requested fields.".to_string(),
                );
            }
            let search_runtime =
                feature_tools::validate_web_search(&self.platform.paths.config_dir, None);
            if let Some(engines) = search_runtime.get("engines").and_then(Value::as_array) {
                let ready_engines = engines
                    .iter()
                    .filter(|entry| entry.get("ready").and_then(Value::as_bool) == Some(true))
                    .filter_map(|entry| entry.get("engine").and_then(Value::as_str))
                    .collect::<Vec<_>>();
                let default_engine = search_runtime
                    .get("default_engine")
                    .and_then(Value::as_str)
                    .unwrap_or("unknown");
                if !ready_engines.is_empty() {
                    inject_context_message(
                        messages,
                        format!(
                            "## Search Runtime Hint\nConfigured default search engine: `{}`. Ready search engines in this runtime: {}. Prefer a ready engine directly when calling `web_search`, and if one query returns weak aggregator pages, broaden the organizer mix instead of downloading multiple events from one host family.",
                            default_engine,
                            ready_engines
                                .iter()
                                .map(|engine| format!("`{}`", engine))
                                .collect::<Vec<_>>()
                                .join(", ")
                        ),
                    );
                }
            }
        }
        if tools.iter().any(|tool| tool.name == "web_search")
            && prompt_requests_numeric_market_fact(prompt)
        {
            inject_context_message(
                messages,
                "## Current Fact Hint\nIf the task asks for a current stock or market price, the saved report must include a concrete numeric quote and an explicit date in a grader-visible format such as `YYYY-MM-DD` or a full month name. If the first search only returns source links or vague snippets, download one cited source page into the workspace and read it with a narrow file_manager pattern plus a small max_chars budget before writing the report. For large HTML or escaped JSON pages, prefer a combined entity-plus-field pattern such as the ticker together with the target field name, and if broad keyword snippets look unrelated, switch to a regex-style pattern that pairs the entity identifier with the numeric field. Once you have one concrete numeric quote and date from a grounded source, write the report immediately instead of continuing to browse. Do not save placeholder text saying the number could not be extracted.".to_string(),
            );
        }
        if prompt_references_grounded_inputs_for_code_generation(prompt) {
            inject_context_message(
                messages,
                "## Grounded Code Contract\nThis code-generation request depends on real input files in the workspace. Read those files first, then write or run code that loads the referenced files at runtime instead of hardcoding extracted values into constants. Prefer local grounded code generation over delegating to `run_coding_agent` for this pattern.".to_string(),
            );
        }
    }
}
