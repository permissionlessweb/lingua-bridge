# Discord

## Create Your Discord Bot

Before deploying, set up your Discord application:

1. Go to the [Discord Developer Portal](https://discord.com/developers/applications)
2. Click **New Application** and name it (e.g., "LinguaBridge")
3. Navigate to **Bot** in the sidebar, then click **Add Bot**
4. Under **Privileged Gateway Intents**, enable:
   - **Message Content Intent** (required to read messages for translation)
5. Click **Reset Token** and copy it somewhere safe (you'll need this for provisioning)
6. Go to **OAuth2 → URL Generator**
7. Under **Scopes**, select:
   - `bot`
   - `applications.commands`
8. Under **Bot Permissions**, select:
   - Read Messages/View Channels
   - Send Messages
   - Embed Links
   - Read Message History
   - Connect (for voice channels)
   - Speak (for TTS playback)
   - Use Voice Activity (to receive audio)
9. Copy the generated URL at the bottom and open it in your browser
10. Select the server to add the bot to and authorize it

Keep your bot token safe - you'll use it during the provisioning step after deployment.

Once the bot is running, use these slash commands in your Discord server:

### Server Setup (Admin)

| Command | Description |
| --------- | ------------- |
| `/setup init` | Initialize LinguaBridge for your server |
| `/setup channel #channel enable:true` | Enable translation in a text channel |
| `/setup languages en,es,fr` | Set target languages for the server |
| `/setup status` | View current configuration |

### Text Translation

| Command | Description |
| --------- | ------------- |
| `/translate text:Hello target:es` | Translate text to a specific language |
| `/languages` | List all supported languages |
| `/mylang es` | Set your preferred language |
| `/mypreferences` | View your current preferences |
| `/webview` | Get a link to the web translation viewer |

### Voice Translation

| Command | Description |
| --------- | ------------- |
| `/voice join [channel]` | Bot joins your voice channel (or specified channel) |
| `/voice leave` | Bot leaves the voice channel |
| `/voice status` | View voice translation status |
| `/voice url [channel]` | Get public web URL for viewing voice transcripts |
| `/voice transcript enable:true [text_channel] [languages]` | Enable transcript posting to Discord threads |
| `/voiceconfig language:es tts:true` | Configure voice channel settings |

### Initial Server Setup

1. Run `/setup init` as a server admin
2. Run `/setup channel #general enable:true` for each text channel to translate
3. Run `/setup languages en,es,fr,de` to set target languages
4. Users run `/mylang <code>` to set their preferred language

### Voice Channel Setup

1. Have the bot join your voice channel with `/voice join`
2. Get the web view URL with `/voice url` and share it with participants
3. Optionally enable Discord thread transcripts with `/voice transcript enable:true languages:en,es,fr`

---

## Discord Thread Transcripts (Moderators)

Moderators can configure the bot to post voice transcripts to Discord threads, creating a searchable archive of voice conversations.

### Setting Up Thread Transcripts

1. Join the voice channel you want to transcribe
2. Run the command:

   ```sh
   /voice transcript enable:true text_channel:#transcripts languages:en,es,fr
   ```

3. The bot creates threads for each language:
   - "Voice Translation - English"
   - "Voice Translation - Spanish"
   - "Voice Translation - French"

### Transcript Format

Messages appear in threads as:

```sh
**Username**
> Original text in source language
Translated text in target language
```

### Managing Transcripts

| Action | Command |
| -------- | --------- |
| Enable transcripts | `/voice transcript enable:true languages:en,es` |
| Disable transcripts | `/voice transcript enable:false` |
| Change text channel | `/voice transcript enable:true text_channel:#new-channel` |
| Add languages | Re-run the command with updated language list |

### Notes

- Threads are created in the specified text channel (or current channel if not specified)
- Each language gets its own thread for organized archives
- Threads auto-archive after 24 hours of inactivity (Discord default)
- Transcripts persist in Discord even if the bot goes offline
