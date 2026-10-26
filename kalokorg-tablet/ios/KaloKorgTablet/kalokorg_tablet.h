#ifndef KALOKORG_TABLET_H
#define KALOKORG_TABLET_H

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/// Launches the KaloKorg M20s iPadOS eframe/egui synthesizer & low-latency CoreAudio stream.
int32_t kalokorg_tablet_ios_main(void);

#ifdef __cplusplus
}
#endif

#endif /* KALOKORG_TABLET_H */
