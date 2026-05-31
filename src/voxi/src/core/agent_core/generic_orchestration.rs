#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AgentTaskIntent {
    SearchItem,
    SelectOption,
    ModifyCart,
    ViewState,
    ShowPaymentOptions,
    FinalizeTransaction,
    CancelOrRedirect,
    Unknown,
}

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProviderStatus {
    Queued,
    Running,
    Done,
    Failed,
    Recovering,
    NeedsInput,
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct AgentTaskState {
    pub task_id: String,
    pub intent: AgentTaskIntent,
    pub current_step: String,
    pub retry_count: usize,
    pub last_tool: Option<String>,
    pub last_error: Option<String>,
    pub next_action: Option<String>,
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct ActionReceipt {
    pub provider: String,
    pub action: String,
    pub status: ProviderStatus,
    pub summary: String,
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct AddressCandidate {
    pub provider: String,
    pub id: String,
    pub label: String,
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct ProductCandidate {
    pub provider: String,
    pub id: String,
    pub label: String,
    pub price_text: Option<String>,
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct CartState {
    pub provider: String,
    pub item_count: usize,
    pub verified: bool,
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct PaymentOption {
    pub provider: String,
    pub id: String,
    pub label: String,
}

#[allow(dead_code)]
pub trait SubtaskRunner {
    fn runner_name(&self) -> &'static str;
}

#[allow(dead_code)]
pub trait ProviderAdapter {
    fn provider_key(&self) -> &'static str;
}

#[allow(dead_code)]
pub struct ToolResultNormalizer;

#[allow(dead_code)]
pub struct RecoveryRunner;

#[allow(dead_code)]
pub struct SelectionResolver;

fn classify_agent_task_intent(prompt: &str) -> AgentTaskIntent {
    let lower = prompt.to_ascii_lowercase();
    if lower.trim().is_empty() {
        return AgentTaskIntent::Unknown;
    }
    if lower.contains("cancel") || lower == "no" || lower.contains("stop") {
        return AgentTaskIntent::CancelOrRedirect;
    }
    if lower.contains("payment option")
        || lower.contains("payment method")
        || lower.contains("can use")
        || lower.contains("pay with")
        || lower.contains("show payment")
    {
        return AgentTaskIntent::ShowPaymentOptions;
    }
    if lower.contains("checkout")
        || lower.contains("place order")
        || lower.contains("confirm order")
        || lower.contains("create order")
    {
        return AgentTaskIntent::FinalizeTransaction;
    }
    if lower.contains("cart")
        || lower.contains("status")
        || lower.contains("show")
        || lower.contains("view")
    {
        return AgentTaskIntent::ViewState;
    }
    if lower.contains("add")
        || lower.contains("qty")
        || lower.contains("quantity")
        || lower.contains("remove")
        || lower.contains("cheapest")
    {
        return AgentTaskIntent::ModifyCart;
    }
    if lower.contains("select")
        || lower.contains("choose")
        || lower.contains("option")
        || lower.contains("zepto")
        || lower.contains("swiggy")
    {
        return AgentTaskIntent::SelectOption;
    }
    if lower.contains("search")
        || lower.contains("find")
        || lower.contains("need")
        || lower.contains("get")
    {
        return AgentTaskIntent::SearchItem;
    }
    AgentTaskIntent::Unknown
}

fn is_payment_read_tool_name(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    lower.contains("get_payment_methods")
        || lower.contains("list_payment_methods")
        || lower.contains("payment_methods")
        || lower.contains("check_payment_status")
}

fn is_transaction_finalize_tool_name(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    if is_payment_read_tool_name(&lower) {
        return false;
    }
    lower.contains("checkout")
        || lower.contains("create_order")
        || lower.contains("place_order")
        || lower.contains("online_payment_order")
        || lower.contains("wallet_order")
        || lower.contains("reserve_pay")
}

fn guarded_tool_reason(intent: AgentTaskIntent, tool_name: &str) -> Option<String> {
    if is_transaction_finalize_tool_name(tool_name)
        && intent != AgentTaskIntent::FinalizeTransaction
    {
        return Some(
            "Checkout or order creation requires an explicit final order request.".to_string(),
        );
    }

    match intent {
        AgentTaskIntent::ShowPaymentOptions if is_transaction_finalize_tool_name(tool_name) => {
            Some(
                "Payment options must be shown before any checkout or order action can run."
                    .to_string(),
            )
        }
        AgentTaskIntent::ViewState if is_transaction_finalize_tool_name(tool_name) => {
            Some("Viewing state cannot trigger checkout or order creation.".to_string())
        }
        _ => None,
    }
}

fn action_signature(session_id: &str, tool_name: &str, args: &Value) -> String {
    let args_text = serde_json::to_string(args).unwrap_or_default();
    format!("{}:{}:{}", session_id, tool_name, args_text)
}
