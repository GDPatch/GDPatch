
#include "gdpatch_os.h"

void GDPatchOS::_bind_methods() {
    ClassDB::bind_static_method("GDPatchOS", D_METHOD("get_stdin_buffer", "buffer_size"), &GDPatchOS::get_stdin_buffer);
}
