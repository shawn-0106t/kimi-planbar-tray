#include "state.h"

// AppState is a plain aggregate mutated only on the GUI thread; worker
// threads compute (fetch/update check/skills scan) and post results back.
