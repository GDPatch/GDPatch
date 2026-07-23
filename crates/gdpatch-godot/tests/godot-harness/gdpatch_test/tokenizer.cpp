#include "tokenizer.h"


void BoundToken::_bind_methods() {
	ClassDB::bind_method(D_METHOD("get_type"), &BoundToken::get_type);
	ADD_PROPERTY(PropertyInfo(Variant::INT, "type"), "", "get_type");

	ClassDB::bind_method(D_METHOD("get_name"), &BoundToken::get_name);
	ADD_PROPERTY(PropertyInfo(Variant::STRING, "name"), "", "get_name");

	ClassDB::bind_method(D_METHOD("get_literal"), &BoundToken::get_literal);
	ADD_PROPERTY(PropertyInfo(Variant::OBJECT, "literal"), "", "get_literal");
}

int BoundToken::get_type() const {
	return this->inner.type;
}

String BoundToken::get_name() const {
	return this->inner.get_name();
}

Variant BoundToken::get_literal() const {
	return this->inner.literal;
}

void BoundGDTokenizerText::_bind_methods() {
	ClassDB::bind_method(D_METHOD("set_source_code", "source"), &BoundGDTokenizerText::set_source_code);
	ClassDB::bind_method(D_METHOD("scan"), &BoundGDTokenizerText::scan);
}

void BoundGDTokenizerText::set_source_code(const String& source) {
	this->inner.set_source_code(source);
}

Ref<BoundToken> BoundGDTokenizerText::scan() {
	const GDScriptTokenizer::Token token = this->inner.scan();
	Ref<BoundToken> bound = memnew(BoundToken);
	bound->inner = token;

	return bound;
}

#ifdef GDPATCH_HAS_TOKENIZER_BUFFER
void BoundGDTokenizerBuffer::_bind_methods() {
	ClassDB::bind_method(D_METHOD("set_code_buffer", "buffer"), &BoundGDTokenizerBuffer::set_code_buffer);
	ClassDB::bind_method(D_METHOD("scan"), &BoundGDTokenizerBuffer::scan);
	ClassDB::bind_static_method("BoundGDTokenizerBuffer", D_METHOD("parse_code_string", "source"), &BoundGDTokenizerBuffer::parse_code_string);
}

PackedByteArray BoundGDTokenizerBuffer::parse_code_string(const String &source) {
	return GDScriptTokenizerBuffer::parse_code_string(source, GDScriptTokenizerBuffer::COMPRESS_NONE);
}

Error BoundGDTokenizerBuffer::set_code_buffer(const Vector<uint8_t> &buffer) {
	return this->inner.set_code_buffer(buffer);
}

Ref<BoundToken> BoundGDTokenizerBuffer::scan() {
	const GDScriptTokenizer::Token token = this->inner.scan();
	Ref<BoundToken> bound = memnew(BoundToken);
	bound->inner = token;

	return bound;
}
#endif
