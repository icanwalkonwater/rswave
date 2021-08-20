use crate::{
    async_app::errors::{ResultSpotify as Result, SpotifyError},
    Opt,
};
use rspotify::{
    client::{ApiError, Spotify},
    model::{
        audio::{AudioAnalysis, AudioAnalysisMeasure},
        playing::Playing,
        track::FullTrack,
    },
    oauth2::{SpotifyClientCredentials, SpotifyOAuth},
};
use std::{
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::{sync::Mutex, task::JoinHandle};

pub struct SpotifyHolder {
    track: Arc<Mutex<Option<TrackHolder>>>,
    handle: JoinHandle<()>,
}

pub struct TrackHolder {
    pub id: String,
    pub name: String,
    pub artist: String,
    pub duration_ms: u32,
    pub progress_ms: u32,
    pub beats: Box<[AudioAnalysisMeasure]>,
    pub tempo: f32,
    pub is_beat: bool,
    last_beat_index: usize,
    query_time: Instant,
}

// TODO: handle errors better than that.
impl SpotifyHolder {
    pub async fn new(opt: Opt) -> Result<Self> {
        let mut oauth = SpotifyOAuth::default()
            .client_id(opt.spotify_id.as_ref().unwrap())
            .client_secret(opt.spotify_secret.as_ref().unwrap())
            .redirect_uri("http://localhost/")
            .scope("user-read-currently-playing")
            .build();

        // Ask for token
        let token = if opt.spotify_auth_fresh {
            rspotify::util::get_token_without_cache(&mut oauth).await
        } else {
            rspotify::util::get_token(&mut oauth).await
        }
        .ok_or(SpotifyError::UnableToGetAccessToken)?;

        let credentials = SpotifyClientCredentials::default()
            .token_info(token)
            .build();

        let spotify = Spotify::default()
            .client_credentials_manager(credentials)
            .build();

        let track = Arc::new(Mutex::<Option<TrackHolder>>::new(None));
        let track_clone = track.clone();
        let handle = tokio::task::spawn(async move {
            Self::run(opt.spotify_refresh_interval, oauth, spotify, track_clone).await;
        });

        Ok(Self { track, handle })
    }

    async fn run(
        refresh_interval: f32, mut oauth: SpotifyOAuth, mut spotify: Spotify,
        shared_track: Arc<Mutex<Option<TrackHolder>>>,
    ) {
        let refresh_interval = Duration::from_secs_f32(refresh_interval);
        let mut last_request_time = Instant::now();
        let mut next_poll_override = Instant::now();

        loop {
            match spotify.current_user_playing_track().await {
                Ok(Some(Playing {
                    item:
                        Some(FullTrack {
                            id: Some(id),
                            name,
                            artists,
                            duration_ms,
                            ..
                        }),
                    progress_ms,
                    ..
                })) => {
                    last_request_time = Instant::now();

                    // Update holder
                    let mut shared_track = shared_track.lock().await;
                    if shared_track.is_some()
                        && shared_track
                            .as_ref()
                            .map(|holder| &holder.id == &id)
                            .unwrap_or(false)
                    {
                        let holder = shared_track.as_mut().unwrap();
                        holder.progress_ms = progress_ms.unwrap_or(0);
                        holder.query_time = last_request_time;
                    } else {
                        let holder = shared_track.as_mut().unwrap();
                        let analysis = spotify.audio_analysis(&id).await.unwrap();

                        *holder = TrackHolder {
                            id,
                            name,
                            artist: artists[0].name.clone(),
                            duration_ms,
                            progress_ms: progress_ms.unwrap_or(0),
                            beats: analysis.beats.into_boxed_slice(),
                            tempo: analysis.track.tempo,
                            is_beat: false,
                            last_beat_index: 0,
                            query_time: last_request_time,
                        };
                    }

                    // Schedule next force poll to the end of the track
                    if let Some(holder) = shared_track.as_ref() {
                        let ms_to_the_end = holder.duration_ms - holder.progress_ms;
                        next_poll_override =
                            last_request_time + Duration::from_millis(ms_to_the_end as u64);
                    }
                }
                Ok(_) => {
                    // No track playing
                    last_request_time = Instant::now();
                    // Empty the track holder
                    *shared_track.lock().await = None;
                }
                Err(err) => {
                    last_request_time = Instant::now();

                    match err.downcast::<ApiError>() {
                        Ok(ApiError::RateLimited(Some(secs))) => {
                            // TODO: Log better than that bro
                            eprintln!("Rate limited for {} secs", secs);

                            // Ensure that no requests are made until the rate limit is over
                            let closest_next_time =
                                last_request_time + Duration::from_secs(secs as u64);
                            if next_poll_override < closest_next_time {
                                next_poll_override = closest_next_time;
                            }
                        }
                        Ok(ApiError::Unauthorized) | Ok(_) => {
                            // Re auth and retry next time
                            let token = rspotify::util::get_token(&mut oauth).await;
                            let cred = spotify
                                .client_credentials_manager
                                .take()
                                .unwrap()
                                .token_info(token.expect("Failed to refresh token"));

                            spotify = Spotify::default().client_credentials_manager(cred).build();

                            // Retry as soon as possible
                            next_poll_override = last_request_time;
                        }
                        Err(err) => panic!(err),
                    }
                }
            };

            // Wait for a bit
            tokio::time::delay_until(
                (last_request_time + refresh_interval)
                    .min(next_poll_override)
                    .into(),
            )
            .await;
        }
    }
}

impl TrackHolder {
    pub fn compute_real_progress_ms(&self) -> u32 {
        self.progress_ms + Instant::now().duration_since(self.query_time).as_millis() as u32
    }

    pub fn advance_beat(&mut self) {
        let progress = self.compute_real_progress_ms() as f32 / 1000.0;

        let beat = self
            .beats
            .iter()
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
