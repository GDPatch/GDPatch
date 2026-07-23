const _READ_SIZE = 1024

static func read_input():
    var input = GDPatchOS.get_stdin_buffer(_READ_SIZE)

    var length_end = input.find(0x0A)

    if length_end == -1:
        push_error('Missing length newline marker in input')
        return null

    var length = int(input.slice(0, length_end).get_string_from_ascii())
    var buffer = input.slice(length_end + 1)

    # check if buffer overread
    if buffer.size() > length:
        buffer = buffer.slice(0, length)

    else:
        # check if we need to read more data
        var remaining_length = length - buffer.size()

        while remaining_length > 0:
            var read = GDPatchOS.get_stdin_buffer(min(remaining_length, _READ_SIZE))

            if read.is_empty():
                break

            remaining_length -= read.size()
            buffer.append_array(read)

    if buffer.size() != length:
        push_error('Read wrong amount of data: wanted ' + str(length) + ', got ' + str(buffer.size()))
        return null

    return buffer

static func emit(value: Variant) -> void:
    var output = GDPatchJson.stringify(value)
    print(output)
