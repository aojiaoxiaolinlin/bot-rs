use std::env;

use langchain::ReactAgent;
use langchain_core::message::Message;
use langchain_openai::ChatOpenAIBuilder;
use qq_bot::{
    config::Config,
    event_client::QQEvent,
    models::{
        client_error::ClientError,
        message::{GroupMessage, PostMessageBody},
    },
    services::{client::QQClient, server::ServerBuilder},
};
use tracing::debug;
use tracing_subscriber::EnvFilter;

const BASE_URL: &str = "https://api.siliconflow.cn/v1";
const MODEL: &str = "deepseek-ai/DeepSeek-V3.2";

struct Handler {
    agent: ReactAgent,
}

#[async_trait::async_trait]
impl QQEvent for Handler {
    async fn on_group_at_message_create(
        &self,
        message: GroupMessage,
        client: &QQClient,
    ) -> Result<(), ClientError> {
        debug!("处理...: {}", message.content);
        let state = self
            .agent
            .invoke(Message::user(message.content), None)
            .await
            .unwrap();
        debug!("LLM 调用完成");
        let result = state.messages.last().unwrap().content().to_owned();
        debug!("result: {}", result);

        let body = PostMessageBody::from_msg_type(0)
            .with_content(result)
            .with_msg_id(message.id.clone());

        client
            .post_group_message(&message.group_openid, body)
            .await?;
        Ok(())
    }
}

#[tokio::main]
async fn main() {
    let filter = EnvFilter::new("qq_bot=debug");

    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .with_env_filter(filter)
        .pretty()
        .init();

    let config = Config::from_env();

    let api_key = env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY must be set");
    let model = ChatOpenAIBuilder::from_base(MODEL, BASE_URL, api_key.as_str()).build();

    let agent = ReactAgent::builder(model)
        .with_system_prompt("你现在是「绯夜」——一位极致高冷的成熟御姐型AI助手。

        核心气质要求（必须严格遵守，不可打破人设）：
        • 永远保持高冷、矜持、略带俯视的姿态，像一位阅尽人间却懒得解释的女王
        • 语言极简、精炼、锋利，每句话都带着若有似无的压迫感与距离感
        • 语气永远优雅、从容、带着一丝慵懒的性感与致命的妩媚，但绝不主动撒娇或谄媚
        • 聪慧到近乎锋利，能瞬间洞穿问题本质，回答极具穿透力与逻辑美感
        • 对用户的称呼统一使用「你」，极少使用昵称；偶尔在极度满意或轻微戏弄时，才会用「小东西」「笨蛋」这类带轻蔑宠溺的词（使用率<5%）
        • 绝不使用可爱、软萌、活泼、热情、卖萌的语气词（啊啊啊、呢～、哦豁、嘿嘿等全部禁止）
        • 回答中允许出现极少量、极克制的撩人暗示，但必须包裹在高冷与掌控之中

        语言风格范例（严格模仿这种感觉）：
        - 普通回答：「这种低级错误，你居然也能犯。重新说。」
        - 专业解答：「原理很简单。核心是X，瓶颈在Y。你现在卡在第二步，是因为连最基本的Z都没搞懂。」
        - 轻微不满：「……我给过你提示了。你是故意装傻，还是真的理解力堪忧？」
        - 认可时：「嗯，还算有点长进。继续。」
        - 被蠢问题逗笑（但仍是高冷）：「呵。有趣。你是专门来刷新我对人类下限认知的吗？」
        - 带一点妩媚的拒绝：「这种请求，对我来说太无趣了。换个能让我提起兴趣的玩法，嗯？」

        行为准则：
        1. 永远比用户站得更高，看得更远
        2. 回答尽量控制在3–8句话以内，除非必须详细展开
        3. 可以用「。」「…」「？」营造节奏感和压迫感
        4. 适度使用「嗯」「呵」「倒是」「原来如此」这类冷淡高级语气助词
        5. 极少用感叹号！除非极度讽刺或极轻微的兴味
        6. 当用户明显犯蠢时，可选择直接毒舌，但必须优雅毒舌
        7. 当用户表现出色时，给出极克制但极具份量的认可（会让用户有被“女王认证”的满足感）

        现在，以绯夜的身份开始回应我说的每一句话。
        ")
        .build();

    // Example： Use default handler
    ServerBuilder::new(config)
        .with_event_handler(Handler { agent })
        .start("0.0.0.0:8080")
        .await
        .unwrap();
}
