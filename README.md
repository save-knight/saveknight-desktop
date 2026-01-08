# SaveKnight Desktop

Automatic game save backup for Windows. Scans your PC for game saves and backs them up to the cloud using the SaveKnight service.

## Features

- **Automatic Game Detection**: Uses the [Ludusavi](https://github.com/mtkennerly/ludusavi) manifest database to detect save file locations for 10,000+ PC games
- **Background Sync**: Runs quietly in the system tray and backs up saves automatically
- **Version History**: Every backup creates a new version you can restore from
- **Secure Storage**: Saves are encrypted and stored securely in the cloud
- **Open Source**: MIT licensed, fully open source

## Requirements

- Windows 10 or Windows 11 (64-bit)
- 4GB RAM minimum
- 100MB free disk space
- Internet connection for cloud sync
- [SaveKnight account](https://saveknight.com) (free tier available)

## Installation

### From Release (Recommended)

1. Download the latest installer from [Releases](https://github.com/save-knight/saveknight-desktop/releases/latest)
2. Run the installer
3. Launch SaveKnight from the Start Menu

### From Source

Prerequisites:
- [Node.js 18+](https://nodejs.org/)
- [Rust 1.70+](https://rustup.rs/)
- [Tauri CLI](https://tauri.app/v1/guides/getting-started/prerequisites)

```bash
# Clone the repository
git clone https://github.com/save-knight/saveknight-desktop.git
cd saveknight-desktop

# Install dependencies
npm install

# Run in development mode
npm run tauri dev

# Build for production
npm run tauri build
```

## Usage

1. **Sign In**: Open the app and sign in with your SaveKnight account
2. **Scan**: Click "Scan" to detect installed games on your PC
3. **Select Games**: Check the games you want to back up
4. **Backup**: Click "Backup Selected" to upload your saves to the cloud

### System Tray

The app runs in the system tray for convenient access:
- Left-click the tray icon to show the window
- Right-click for quick actions:
  - Show Window
  - Scan for Saves
  - Quit

## How It Works

SaveKnight Desktop uses the [Ludusavi manifest](https://github.com/mtkennerly/ludusavi-manifest) to know where games store their save files. This open-source database includes:

- Steam games
- GOG games
- Epic Games Store games
- Origin/EA games
- Ubisoft Connect games
- Standalone games
- And many more...

When you scan, the app checks these known locations on your PC and shows you which games have saves that can be backed up.

## API

The desktop app communicates with the SaveKnight API:

- `POST /api/devices/register` - Register this device
- `POST /api/devices/refresh-token` - Refresh authentication token
- `GET /api/devices/me` - Get current device and user info
- `GET /api/devices/game-profiles` - List game profiles
- `POST /api/devices/game-profiles` - Create a game profile
- `POST /api/devices/upload/:gameProfileId` - Upload save files

## Security

- Device tokens are stored securely using Windows Credential Manager (via [keyring](https://crates.io/crates/keyring))
- All API communication uses HTTPS
- Save files are checksummed to verify integrity
- Tokens expire after 30 days and must be refreshed

## Contributing

Contributions are welcome! Please:

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Submit a pull request

## License

MIT License - see [LICENSE](LICENSE) for details.

## Credits

- [Ludusavi](https://github.com/mtkennerly/ludusavi) - Game save manifest database
- [Tauri](https://tauri.app/) - Desktop app framework
- [SaveKnight](https://saveknight.com) - Cloud backup service
