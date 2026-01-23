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
