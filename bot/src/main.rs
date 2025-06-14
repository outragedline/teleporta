use config::{Config, Environment};
use rumqttc::{AsyncClient, MqttOptions, QoS};
use serde::{Deserialize, Serialize};
use teloxide::{prelude::*, utils::command::BotCommands};
use tokio::sync::mpsc;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MqttSettings {
    device_id: String,
    host: String,
    port: u16,
    username: String,
    password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BotSettings {
    chat_id: i64,
    token: String,
    mqtt: MqttSettings,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    let settings = Config::builder()
        .add_source(config::File::with_name("config.toml"))
        .add_source(Environment::with_prefix("APP"))
        .build()?;

    let settings = settings.try_deserialize::<BotSettings>()?;

    let chat_id = ChatId(settings.chat_id);

    let bot = Bot::new(settings.token);

    let mut mqtt_options = MqttOptions::new(
        settings.mqtt.device_id,
        settings.mqtt.host,
        settings.mqtt.port,
    );
    mqtt_options
        .set_credentials(settings.mqtt.username, settings.mqtt.password)
        .set_transport(rumqttc::Transport::tls_with_default_config());

    let (async_client, mut event_loop) = AsyncClient::new(mqtt_options, 10);
    let (tx, mut rx) = mpsc::channel::<(String, String)>(100);

    async_client
        .subscribe("door/alert", QoS::AtMostOnce)
        .await
        .unwrap();

    {
        let bot = bot.clone();
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    evres = event_loop.poll() => {
                        match evres{
                            Ok(rumqttc::Event::Incoming(rumqttc::Packet::Publish(_)))=>{
                                if let Err(e) = bot.send_message(chat_id, "Someone's at the door").await{
                                    println!("{e}")
                                };
                            },
                            Ok(_)=> {},
                            Err(e) => {
                                println!("Here it is");
                                println!("{e}");
                            },
                        }
                    }
                    Some((topic, payload)) = rx.recv() => {
                        if let Err(e) = async_client.publish(topic.clone(), QoS::AtMostOnce, false, payload.as_bytes()).await {
                            println!("{e:?}");
                        };
                    }
                }
            }
        });
    }

    Dispatcher::builder(
        bot,
        dptree::entry().branch(
            Update::filter_message()
                .filter_command::<Command>()
                .endpoint(answer),
        ),
    )
    .dependencies(dptree::deps![tx])
    .enable_ctrlc_handler()
    .build()
    .dispatch()
    .await;

    Ok(())
}

#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase", description = "Commands")]
enum Command {
    #[command(description = "Open the door")]
    Approve,
    #[command(description = "Play sad music")]
    Disapprove,
    #[command(description = "Yes im helping")]
    Help,
}

async fn answer(
    bot: Bot,
    msg: Message,
    cmd: Command,
    tx: mpsc::Sender<(String, String)>,
) -> ResponseResult<()> {
    match cmd {
        Command::Approve => {
            let _ = tx.send(("door/approved".to_string(), "".to_string())).await;
        }
        Command::Disapprove => {
            let _ = tx
                .send(("door/disapproved".to_string(), "".to_string()))
                .await;
        }
        Command::Help => {
            bot.send_message(msg.chat.id, Command::descriptions().to_string())
                .await?;
        }
    }
    Ok(())
}
