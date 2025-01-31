use chrono::Local;
use dotenv::dotenv;
use std::{env, error::Error};
use teloxide::types::Seconds;
use teloxide::{
    dptree::case,
    macros::BotCommands,
    net::Download,
    prelude::*,
    types::{ChatAction, InputFile},
    Bot,
};
use tokio::fs;

async fn start(bot: Bot, msg: Message) -> ResponseResult<()> {
    const RULES: &str = r#"ПРАВИЛА:
видео не должно быть больше 360p
видео должно быть квадратным
видео должно быть короче 60 секунд
видео не должно быть тяжелее 8 мегабайт"#;
    let text;
    if let Some(first_name) = msg.chat.first_name() {
        text = format!("привет, {first_name}, пришли видео\n{RULES}");
    } else {
        text = format!("привет, пришли видео\n{RULES}");
    }
    bot.send_message(msg.chat.id, text).await?;
    println!("@{}, /start", msg.chat.username().unwrap_or_default());
    Ok(())
}

async fn handle_video_message(bot: Bot, msg: Message) -> ResponseResult<()> {
    let video = msg.video().unwrap();

    if video.width > 640 || video.height > 640 {
        bot.send_message(msg.chat.id, "видео не должно быть больше 360p:(")
            .await?;
    } else if video.width != video.height {
        bot.send_message(msg.chat.id, "видео должно быть квадратным:(").await?;
    } else if video.duration > Seconds::from_seconds(60) {
        bot.send_message(msg.chat.id, "видео должно быть короче 60 секунд:(")
            .await?;
    } else if (video.file.size / 1024 / 1024) > 8 {
        bot.send_message(msg.chat.id, "видео не должно быть тяжелее 8 мегабайт:(")
            .await?;
    } else {
        let now = Local::now();
        let file = bot.get_file(&video.file.id).await?;
        let file_path = msg.chat.username().unwrap_or_default();

        let path = format!("./videos/{file_path}-{now}.mp4");
        let mut dst = fs::File::create(&path).await?;
        bot.download_file(&file.path, &mut dst).await?;

        let video_note = InputFile::file(&path);
        bot.send_chat_action(msg.chat.id, ChatAction::UploadVideoNote).await?;
        bot.send_video_note(msg.chat.id, video_note).await.expect("");
        println!(
            "@{}, id={}, {}",
            msg.chat.username().unwrap_or_default(),
            msg.chat.id,
            path
        );
    }

    Ok(())
}

async fn handle_non_videos(bot: Bot, msg: Message) -> ResponseResult<()> {
    bot.send_message(msg.chat.id, "не понял тебя. пришли видео!").await?;
    println!(
        "@{}, id={}, {}",
        msg.chat.username().unwrap_or_default(),
        msg.chat.id,
        msg.text().unwrap_or_default()
    );

    Ok(())
}

#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase")]
enum Command {
    #[command(description = "show help.")]
    Start,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    dotenv().ok();

    fs::create_dir_all("videos").await?;

    let bot_token = env::var("TELOXIDE_TOKEN").expect("TELOXIDE_TOKEN is not set");
    let bot = Bot::new(bot_token);

    let command_handler = teloxide::filter_command::<Command, _>().branch(case![Command::Start].endpoint(start));

    let handler = dptree::entry().branch(
        Update::filter_message()
            .branch(command_handler)
            .branch(dptree::filter(|msg: Message| msg.video().is_some()).endpoint(handle_video_message))
            .endpoint(handle_non_videos),
    );

    Dispatcher::builder(bot, handler)
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;

    Ok(())
}
