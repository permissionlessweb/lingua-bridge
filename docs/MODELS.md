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

| Model Name | Creator/Organization | Parameters | Key Languages/Features | Notes |
| ------------ | ------------------------- | -------------- | --------- |--------- |
| **Dedicated MT Models** | | | | |
| NLLB-200-3.3B | Meta (Facebook) | 3.3B | 200+ languages, including low-resource ones like African and Asian dialects | No Language Left Behind; state-of-the-art for multilingual translation; distilled versions available for efficiency.  |
| NLLB-200-distilled-600M | Meta (Facebook) / Xenova | 600M | 200+ languages | Lightweight distilled version of NLLB-200; optimized for on-device use. |
| SeamlessM4T | Meta | Varies (up to 7B) | 100+ languages for text-to-text, speech-to-text, etc. | Multimodal (text + speech); supports seamless translation across modalities. |
| M2M-100 | Meta (Facebook) | 12B / 1.2B distilled | 100 languages | Many-to-Many Multilingual Translation; direct any-to-any without English pivot. |
| OPUS-MT (various, e.g., opus-mt-ru-en, opus-mt-en-ru, opus-mt-tc-big-en-tr) | Helsinki-NLP (University of Helsinki) | ~300M-600M per model | Bilingual pairs (e.g., Russian-English, English-Turkish); covers 100+ languages across models | Large collection of bilingual models trained on OPUS dataset; highly specialized. |
| HY-MT1.5-1.8B / HY-MT1.5-7B / Hunyuan-MT-7B / Hunyuan-MT-Chimera-7B | Tencent | 1.8B-7B | 100+ languages, strong in Chinese-English and multilingual | High-quality MT from Tencent; includes GGUF/FP8 quantized versions for efficiency. |
| Plamo-2-Translate | PFNet | 10B | Multilingual, focused on Japanese-English and others | Specialized for accurate translation in Asian languages. |
| Sarvam-Translate | Sarvam AI | 4B | Indian languages + English (e.g., Hindi, Tamil) | Optimized for Indic languages; supports low-resource dialects. |
| SauerkrautLM-Translator-LFM2.5-1.2B | VAGOsolutions | 1.2B | Multilingual, German-focused but broad | Fine-tuned for high-fidelity translation; lightweight. |
| MT5-Sinhalese-English | Thilina | Varies | Sinhalese-English bilingual | Based on mT5; specialized for Sinhala language. |
| XCOMET-XL | Unbabel | Varies | Multilingual evaluation and translation | Focuses on quality estimation alongside translation. |
| Hibiki-1B | Kyutai | 1B | Japanese + multilingual | Strong in Japanese translation; PyTorch-based. |
| Masrawy-English-to-Egyptian-Arabic-Translator-v2.9 | NAMAA-Space | 200M | English to Egyptian Arabic | Dialect-specific; useful for regional Arabic variants. |
| **Multilingual LLMs for Translation** | | | | |
| Llama 3.1-8B-Instruct / Llama 3.1-70B-Instruct | Meta | 8B-70B | 100+ languages via prompting | Excellent for translation; supports low-resource languages better than predecessors.  |
| Qwen2.5-72B-Instruct / Qwen3-235B-A22B | Alibaba | 72B-235B | 100+ languages, strong in Chinese and multilingual | Top performer in translation benchmarks; handles complex queries.  |
| Mistral-Nemo-Instruct-2407 / Mixtral-8x22B-Instruct-v0.1 | Mistral AI | 12B-8x22B | 100+ languages | Fast and efficient; great for European languages and code-switching.  |
| Phi-3.5-mini-instruct | Microsoft | 3.8B | Multilingual via prompting | Lightweight and accurate; open under MIT license. |
| DeepSeek-V2.5 | DeepSeek | 236B | Strong in Chinese-English and broad multilingual | Cost-effective for large-scale translation. |
| Yi-1.5-34B-Chat | 01.AI | 34B | Multilingual, Chinese-focused | High performance in Asian languages. |
| Command R+ | Cohere | 104B | 100+ languages | Optimized for long-context translation and RAG.  |
| Falcon 2 | Technology Innovation Institute | 11B-180B | Multilingual | Strong in Arabic and Middle Eastern languages.  |
| BLOOM | BigScience (Hugging Face) | 176B | 46 languages natively | Early large multilingual model; good for research.  |
| StepFun Step3 | StepFun | Varies | Multilingual | Emerging model with strong translation capabilities.  |
| **Frameworks & Toolkits with Pre-trained Models** | | | | |
| OpenNMT | SYSTRAN / Harvard NLP | Varies | Custom trainable; pre-trained models for many pairs | Neural MT framework; used in production systems. |
| MarianMT | Marian NMT Team | Varies | Bilingual/multilingual; fast C++ implementation | High-speed translation; models available on HF. |
| Bergamot | Mozilla | Varies | European languages primarily | Browser-integrated; local/offline translation. |
| Apertium | Apertium Project | Rule-based | 50+ language pairs | Rule-based (not neural); lightweight for specific pairs like Romance languages. |
| Argos Translate / LibreTranslate | Argos / LibreTranslate | Varies (uses OpenNMT) | 20+ languages offline | API/server for local translation; easy to deploy.  |
| Moses | Moses SMT | Statistical | Legacy pairs | Older statistical MT; still useful for custom training. |

For deployment, most of these are available on Hugging Face for easy fine-tuning or inference. If you need details on a specific model, setup instructions, or benchmarks for certain language pairs, let me know!
