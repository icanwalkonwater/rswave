pub mod net;
pub mod app;
pub mod audio_collector;
pub mod audio_processor;

pub mod errors {
    use thiserror::Error;

    pub type Result<T> = std::result::Result<T, RsWaveError>;
    pub type ResultAudioCollector<T> = std::result::Result<T, AudioCollectorError>;
    pub type ResultNet<T> = std::result::Result<T, NetError>;

    #[derive(Debug, Error)]
    pub enum RsWaveError {
        #[error(transparent)]
        AudioCollectorError(#[from] AudioCollectorError),
        #[error(transparent)]
        NetError(#[from] NetError),
    }

    #[derive(Debug, Error)]
    pub enum AudioCollectorError {
        #[error("Can't find audio device !")]
        AudioDeviceNotFound,
        #[error(transparent)]
        CpalDevicesError(#[from] cpal::DevicesError),
        #[error("Unsupported sample rate, only 44100 Hz is supported !")]
        UnsupportedSampleRate,
        #[error("Only stereo devices are supported !")]
        NotStereoDevice,
        #[error(transparent)]
        CpalDefaultStreamConfigError(#[from] cpal::DefaultStreamConfigError),
        #[error(transparent)]
        CpalStreamError(#[from] cpal::StreamError),
        #[error(transparent)]
        CpalBuildStreamError(#[from] cpal::BuildStreamError),
        #[error("Failed to stop audio collector !")]
        FailedToStopTask,
    }

    #[derive(Debug, Error)]
    pub enum NetError {
        #[error("Hey")]
        Hey
    }
}
