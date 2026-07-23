#include "gdpatch_os.h"
#include "core/object/class_db.h"

#define WIN32_LEAN_AND_MEAN
#include <windows.h>

PackedByteArray GDPatchOS::get_stdin_buffer(int64_t p_buffer_size) {
	Vector<uint8_t> data;
	data.resize_uninitialized(p_buffer_size);
	DWORD count = 0;
	if (ReadFile(GetStdHandle(STD_INPUT_HANDLE), data.ptrw(), data.size(), &count, nullptr)) {
	    data.resize(count);
		return data;
	}

	return {};
}