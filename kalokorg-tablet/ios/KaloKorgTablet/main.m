#import <UIKit/UIKit.h>
#import <AVFoundation/AVFoundation.h>
#include "kalokorg_tablet.h"

/// Configures the iPadOS AVAudioSession for sub-10ms low-latency stereo synthesis (256 frames @ 48 kHz)
/// and hands execution to the Rust `kalokorg-tablet` crate (`kalokorg_tablet_ios_main`).
static void configure_ipados_low_latency_audio(void) {
    AVAudioSession *session = [AVAudioSession sharedInstance];
    NSError *error = nil;

    [session setCategory:AVAudioSessionCategoryPlayback
             withOptions:AVAudioSessionCategoryOptionMixWithOthers
                   error:&error];

    // Request 48,000 Hz hardware sample rate and 256-sample (~5.33 ms) I/O buffer duration
    [session setPreferredSampleRate:48000.0 error:&error];
    [session setPreferredIOBufferDuration:(256.0 / 48000.0) error:&error];
    [session setActive:YES error:&error];
}

int main(int argc, char * argv[]) {
    @autoreleasepool {
        configure_ipados_low_latency_audio();
        return kalokorg_tablet_ios_main();
    }
}
