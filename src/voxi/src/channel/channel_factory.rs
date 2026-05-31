//! Channel factory — creates channel instances from config.

use super::{Channel, ChannelConfig};

pub fn create_channel(
    config: &ChannelConfig,
    agent: Option<std::sync::Arc<crate::core::agent_core::AgentCore>>,
) -> Option<Box<dyn Channel + Send + Sync>> {
    match config.channel_type.as_str() {
        "web_dashboard" => Some(Box::new(super::web_dashboard::WebDashboard::new(config))),
        "tv" => Some(Box::new(super::tv_channel::TvChannel::new(config))),
        "webhook" => Some(Box::new(super::webhook_channel::WebhookChannel::new(
            config,
        ))),
        "telegram" => Some(Box::new(super::telegram_client::TelegramClient::new(
            config,
            agent.clone(),
        ))),
        "discord" => Some(Box::new(super::discord_channel::DiscordChannel::new(
            config,
        ))),
        "slack" => Some(Box::new(super::slack_channel::SlackChannel::new(config))),
        "voice" => Some(Box::new(super::voice_channel::VoiceChannel::new(
            config,
            agent.clone(),
        ))),
        "a2a" => {
            if let Some(a) = agent {
                Some(Box::new(super::a2a_handler::A2aHandler::new(config, a)))
            } else {
                log::warn!("AgentCore missing for a2a channel instantiation");
                None
            }
        }
        _ => {
            log::warn!("Unknown channel type: {}", config.channel_type);
            None
        }
    }
}
