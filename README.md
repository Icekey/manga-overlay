# Manga Overlay

## 📖 Overview

Manga Overlay is a desktop application designed to enhance the experience of reading Japanese manga by providing
real-time translation and kanji lookup capabilities. The application creates a transparent overlay on your screen that
can detect and translate Japanese text from manga or any visual content.

![overlay.png](assets/overlay.png)

**Key Benefits:**

- Instantly look up kanji meanings without switching applications
- Get translations for entire text blocks with a single click
- Track your kanji learning progress with built-in statistics
- Seamlessly integrate with your manga reading workflow

**Current Platform Support:** Windows only

## ✨ Features

- **Text Detection**: Automatically identifies Japanese text in selected screen areas
- **Kanji Lookup**: Provides meanings and readings for individual kanji characters
- **Translation**: Translates detected text using Google Translate (results cached locally)
- **Mouse Passthrough**: Interact with underlying applications while the overlay remains active
- **History Tracking**: Review previously detected text and translations
- **Statistics**: Track frequently viewed kanji to monitor learning progress
- **Customizable Interface**: Adjust transparency, size, and behavior of the overlay
- **CUDA Acceleration**: Optional GPU acceleration for faster text detection

## 🚀 Setup

### Prerequisites

- Windows operating system
- [Rust](https://www.rust-lang.org/tools/install) (latest stable version)
- [Git](https://git-scm.com/downloads/win) for cloning the repository
- [CUDA Toolkit](https://developer.nvidia.com/cuda-downloads) (optional, for GPU acceleration)

### Installation

1. Clone the repository:
   ```
   git clone https://github.com/Icekey/manga-overlay.git
   cd manga-overlay
   ```

2. Build the application:
   ```
   cargo build --release
   ```

3. Run the application:
   ```
   cargo run --release
   ```

   Alternatively, you can run the executable directly from `target/release/manga_overlay.exe`

## 🎮 Usage

### Basic Operation

1. **Select Text Area**: Click and drag on the overlay background to select an area containing Japanese text
2. **View Detected Text**: Hover over blue rectangles to see the detected text
3. **Look Up Kanji**: Scroll while hovering over text to see meanings of individual kanji
4. **Translate Text**: Left-click on a text rectangle to translate the entire text block
5. **Pin Information**: Right-click on a text rectangle to keep the information box open

### Advanced Features

- **Mouse Passthrough**: Enable "Mouse Passthrough" in settings to interact with applications beneath the overlay
- **Auto Restart**: Combined with mouse passthrough, enables continuous text detection
- **History View**: Enable "Show History" to view previously detected text
- **Statistics**: Enable "Show Statistics" to track frequently viewed kanji

### Tips

- OCR is paused while hovering over detected text rectangles
- Translations are cached in a local SQLite database for faster retrieval
- Adjust the zoom factor in settings if the interface is too small or large

## 🔧 Troubleshooting

### Common Issues

- **Text Detection Not Working**: Ensure the selected area has clear, readable text
- **Slow Performance**: Enable CUDA acceleration if you have a compatible NVIDIA GPU
- **Application Not Starting**: Check log files in the `log` directory for error messages

### Log Files

Log files are stored in the `log` directory and can help diagnose issues:

- `manga_overlay.log`: General application logs

## 🤝 Contributing

Contributions are welcome! If you'd like to contribute:

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/new-feature`)
3. Commit your changes (`git commit -m 'Add a useful feature'`)
4. Push to the branch (`git push origin feature/new-feature`)
5. Open a Pull Request

## 📄 License

This project is licensed under the GPL-3.0 License - see the [LICENSE](LICENSE) file for details.

## 🙏 Acknowledgments

This project leverages several notable open-source projects:

- [egui](https://github.com/emilk/egui) - Immediate mode GUI library for creating the overlay
- [kanji-data](https://github.com/davidluzgouveia/kanji-data) - Comprehensive kanji meaning dataset
- [comic-text-detector](https://github.com/dmMaze/comic-text-detector) - Model for manga text box detection
- [manga-ocr](https://github.com/kha-white/manga-ocr) - OCR model specialized for manga text
- [koharu](https://github.com/mayocream/koharu) - ONNX models and scripts for Japanese text detection
- [ort](https://github.com/pykeio/ort) - ONNX Runtime for machine learning inference
- [rusqlite](https://github.com/rusqlite/rusqlite) - SQLite bindings for Rust
