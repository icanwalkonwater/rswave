use anyhow::anyhow;
use anyhow::Result;
use rspotify::client::Spotify;
use rspotify::model::playing::Playing;
use rspotify::oauth2::{SpotifyClientCredentials, SpotifyOAuth};
use std::time::{Duration, Instant};
use rspotify::model::track::FullTrack;
use rspotify::model::audio::AudioAnalysis;

const REGULAR_TIMEOUT_THRESHOLD: Duration = Duration::from_secs(5);

pub struct SpotifyTracker {
    spotify: Spotify,

    // Current track tracking
    last_track_query: Instant,
    track_end_time: Instant,
    current_track_cache: Option<Playing>,

    // Track analysis
    audio_analysis: Option<AudioAnalysis>,
    last_beat_index: usize,
    is_beat: bool,
}

impl SpotifyTracker {
    pub async fn new(client_id: &str, client_secret: &str) -> Result<Self> {
        let mut oauth = SpotifyOAuth::default()
            .client_id(client_id)
            .client_secret(client_secret)
            .redirect_uri("http://localhost/")
            .scope("user-read-currently-playing")
            .build();

        // Ask for token (or get it from cache)
        let token = rspotify::util::get_token(&mut oauth)
            .await
            .ok_or(anyhow!("Failed to get spotify token !"))?;

        let credentials = SpotifyClientCredentials::default()
            .token_info(token)
            .build();

        let spotify = Spotify::default()
            .client_credentials_manager(credentials)
            .build();

        Ok(Self {
            spotify,
            last_track_query: Instant::now() - Duration::from_secs(60),
            track_end_time: Instant::now() - Duration::from_secs(60),
            current_track_cache: None,

            audio_analysis: None,
            last_beat_index: 0,
            is_beat: false,
        })
    }
}

// Current track fetch
impl SpotifyTracker {
    pub async fn refresh_current_track(&mut self) {
        let now = Instant::now();
        if now >= self.track_end_time
            || now.duration_since(self.last_track_query) >= REGULAR_TIMEOUT_THRESHOLD
        {
            // Take several ms
            if let Ok(new_track) = self.spotify.current_user_playing_track().await {
                let mut refresh_analysis = false;
                if let Some(Playing { item: Some(FullTrack { id: Some(new_id), .. }), ..}) = new_track.as_ref() {
                    if let Some(Playing { item: Some(FullTrack { id: Some(old_id), ..}), .. }) = self.current_track_cache.as_ref() {
                        if new_id != old_id {
                            refresh_analysis = true;
                        }
                    } else {
                        refresh_analysis = true;
                    }
                } else {
                    if self.current_track_cache.is_some() {
                        refresh_analysis = true;
                    }
                }

                self.current_track_cache = new_track;
                if refresh_analysis {
                    self.refresh_track_analysis().await;
                }
            } else {
                eprintln!("Request failed for some reason, maybe rate limited");
            }

            let now = Instant::now();
            self.last_track_query = now;
            if let Some(Playing {
                            item: Some(track),
                            progress_ms: Some(progress_ms),
                            ..
                        }) = self.current_track_cache.as_ref()
            {
                self.track_end_time =
                    now + Duration::from_millis((track.duration_ms - progress_ms) as u64);
            }
        }
    }

    /// Be sure to call [refresh_current_track] before.
    /// Returns the playing track and its real progress in ms.
    pub fn current_track(&self) -> Option<(&Playing, u32)> {
        if let Some(playing) = self.current_track_cache.as_ref() {
            Some((
                playing,
                self.compute_real_progress_ms(playing),
            ))
        } else {
            None
        }
    }

    #[inline]
    fn compute_real_progress_ms(&self, playing: &Playing) -> u32 {
        playing.progress_ms.unwrap_or(0) + Instant::now().duration_since(self.last_track_query).as_millis() as u32
    }
}

// Track analysis fetch
impl SpotifyTracker {
    async fn refresh_track_analysis(&mut self) {
        if let Some(Playing { item: Some(FullTrack { id: Some(id), .. }), .. }) = self.current_track_cache.as_ref() {
            self.audio_analysis = Some(self.spotify.audio_analysis(id).await.unwrap());
            self.last_beat_index = 0;
        }
    }

    pub fn tempo(&self) -> f32 {
        if let Some(analysis) = self.audio_analysis.as_ref() {
            analysis.track.tempo
        } else {
            f32::MAX
        }
    }

    pub fn advance_beat(&mut self) {
        if let Some(analysis) = self.audio_analysis.as_ref() {
            // If there is an analysis, there is a track
            let progress = self.compute_real_progress_ms(self.current_track_cache.as_ref().unwrap()) as f32 / 1000.0;

            let beat = analysis.beats.iter()
                .enumerate()
                .skip(self.last_beat_index)
                .skip_while(|(_, beat)| beat.start < progress)
                .nth(0);

            if let Some((i, _)) = beat {
                if i != self.last_beat_index {
                    self.is_beat = true;
                    self.last_beat_index = i;
                } else {
                    self.is_beat = false;
                }
            } else {
                self.is_beat = false;
            }
        }
    }

    /// Be sure to call [advance_beat] before to be up to date.
    pub fn is_beat(&self) -> bool {
        self.is_beat
    }
}
