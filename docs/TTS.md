# TTS

## Open Source Text-to-Speech (TTS) Models

Based on various sources, the following is a comprehensive list of notable open source TTS models as of early 2026. This includes both standalone models and toolkits that provide pre-trained models. Models are listed with their primary developer or organization, key features, and license where available. Note that the landscape evolves rapidly, and new models may emerge.

| Model Name | Developer/Organization | Key Features | License |
| ------------ | ------------------------- | -------------- | --------- |
| XTTS-v2 | Coqui AI (now community-maintained) | Multilingual voice cloning from 6-second clips, supports 17+ languages, high-quality synthesis, zero-shot capabilities | Apache 2.0 |
| Higgs Audio V2 | Boson AI | 5.77B parameters, high-fidelity audio generation | Apache 2.0 |
| Kokoro (e.g., v1.0 82M, Kokoro-82M) | Hexgrad | Efficient, 82M parameters, comparable to larger models, cost-effective, supports real-time inference | Apache 2.0 |
| Orpheus (3B/1B/400M/150M variants) | Canopy Labs | Multi-scale architecture, expressive speech, suitable for various sizes | Apache 2.0 |
| Sesame CSM (1B) | Sesame | Optimized for conversational speech | Apache 2.0 |
| Chatterbox | Resemble AI | Multilingual, zero-shot cloning from 5-second samples, real-time, <200ms latency, emotion prompts | MIT |
| MaryTTS | MaryTTS Team | Modular, supports multiple languages and voices | LGPL |
| eSpeak / eSpeak NG | eSpeak Team | Compact, multilingual, lightweight for embedded systems | GPL |
| Festival Speech Synthesis System | University of Edinburgh | Framework for building TTS systems, multilingual | BSD-like |
| Larynx | Rhasspy | Offline-capable, neural network-based | MIT |
| Mozilla TTS | Mozilla (now Coqui) | Flexible, supports Tacotron, Glow-TTS, etc., research-oriented | Mozilla Public License |
| ChatTTS | Community (GitHub) | Optimized for real-time conversation, lightweight | MIT |
| MeloTTS | MyShell.ai | Multilingual (English accents, Chinese/English mix), CPU-optimized, real-time | MIT |
| Mimic 3 | Mycroft AI | Fast, privacy-focused, offline, embedded-friendly | Apache 2.0 |
| Bark | Suno AI | Expressive, includes non-speech sounds (laughter, etc.), creative generation | MIT |
| GPT-SoVITS | Community (RVC-Boss) | Voice cloning, emotional synthesis | MIT |
| Fish Audio / Fish Speech | Fish Audio | High-quality, natural speech | MIT |
| F5-TTS | SWivid | Efficient synthesis | MIT |
| StyleTTS 2 | Community | Style transfer, expressive | MIT |
| Tortoise TTS | Neonbjb | High-fidelity, slow but detailed | MIT |
| Supertonic-2 | Supertone | Advanced synthesis | Unknown |
| Soprano (1.1-80M, 80M) | Ekwek | Lightweight, efficient | Unknown |
| Fun-CosyVoice3-0.5B-2512 | FunAudioLLM | Multilingual, cozy voice style | Unknown |
| IndexTTS / IndexTTS-2 | IndexTeam | Duration control, zero-shot, autoregressive, emotionally expressive | Apache 2.0 |
| Dia (1.6B) | Nari Labs | Podcast-style, multi-speaker, voice cloning | Apache 2.0 |
| VibeVoice | Unknown | Empathetic, high-quality | Unknown |
| OpenAudio | Unknown | General-purpose audio synthesis | Unknown |
| Tango | Community | Text-to-audio, expressive | MIT |
| Stable Audio Open 1.0 | Stability AI | Text-to-audio, music elements | MIT |
| MusicGen | Meta | Music generation with speech elements | MIT |
| Coqui TTS (toolkit with models like Tacotron, Glow-TTS, FastSpeech, VITS, HiFi-GAN, WaveRNN) | Coqui AI | Broad toolkit, multilingual (1,100+ languages), voice cloning | MPL 2.0 |

### Open Source Speech-to-Text (STT) Models

Similarly, here is a comprehensive list of notable open source STT (also known as ASR - Automatic Speech Recognition) models and toolkits. These focus on transcription and sometimes translation.

| Model Name | Developer/Organization | Key Features | License |
| ------------ | ------------------------- | -------------- | --------- |
| Whisper (variants: Large v3 Turbo, Large v3, Large v2, Distil-Whisper, Whisper-small/large) | OpenAI | Multilingual (99 languages), robust to noise/accents, translation, high accuracy (e.g., 10-12% WER), runs on various hardware | MIT |
| Canary Qwen 2.5B | Nvidia | Multilingual, low WER (5.63%), high speed (418x RTFx), speech translation | Apache 2.0 |
| NeMo Canary-1B | Nvidia | Multilingual, 1B parameters, speech-to-text and translation | Apache 2.0 |
| Granite Speech 3.3 | IBM | 8B parameters, 5.85% WER, suitable for enterprise | Apache 2.0 |
| Parakeet TDT 0.6B V2 | Nvidia | 600M parameters, 6.05% WER, ultra-fast (3386x RTFx) | CC-BY-4.0 |
| Kyutai 2.6B (Moshi) | Kyutai | 6.4% WER, 88x RTFx, multilingual | CC-BY-4.0 |
| Reverb ASR | Rev.ai | Trained on 200k hours, high accuracy for English | Unknown (open source) |
| WhisperX | Community | Extension of Whisper, improved alignment and speed | MIT |
| DeepSpeech | Mozilla / Baidu | End-to-end, embedded, real-time on devices (inactive maintenance) | MPL |
| Kaldi | Community | Toolkit for research, flexible, widely used | Apache |
| SpeechBrain | Community | PyTorch-based, Hugging Face integration, research implementations | Apache 2.0 |
| Wav2Vec (e.g., Wav2Vec 2.0) | Meta (Facebook) | Self-supervised, multilingual, fine-tunable | MIT |
| Flashlight ASR (formerly Wav2Letter++) | Meta | Fast, end-to-end, research-focused | MIT |
| Julius | Community | Large vocabulary continuous speech recognition | BSD |
| PaddleSpeech (formerly DeepSpeech2) | Baidu | Toolkit for STT and TTS, multilingual | Apache 2.0 |
