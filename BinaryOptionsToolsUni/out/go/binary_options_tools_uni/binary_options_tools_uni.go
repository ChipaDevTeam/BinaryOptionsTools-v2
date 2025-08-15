
package binary_options_tools_uni

// #include <binary_options_tools_uni.h>
import "C"

import (
	"bytes"
	"fmt"
	"io"
	"unsafe"
	"encoding/binary"
	"runtime/cgo"
	"math"
	"runtime"
	"sync/atomic"
)



// This is needed, because as of go 1.24
// type RustBuffer C.RustBuffer cannot have methods,
// RustBuffer is treated as non-local type
type GoRustBuffer struct {
	inner C.RustBuffer
}

type RustBufferI interface {
	AsReader() *bytes.Reader
	Free()
	ToGoBytes() []byte
	Data() unsafe.Pointer
	Len() uint64
	Capacity() uint64
}

func RustBufferFromExternal(b RustBufferI) GoRustBuffer {
	return GoRustBuffer {
		inner: C.RustBuffer {
			capacity: C.uint64_t(b.Capacity()),
			len: C.uint64_t(b.Len()),
			data: (*C.uchar)(b.Data()),
		},
	}
}

func (cb GoRustBuffer) Capacity() uint64 {
	return uint64(cb.inner.capacity)
}

func (cb GoRustBuffer) Len() uint64 {
	return uint64(cb.inner.len)
}

func (cb GoRustBuffer) Data() unsafe.Pointer {
	return unsafe.Pointer(cb.inner.data)
}

func (cb GoRustBuffer) AsReader() *bytes.Reader {
	b := unsafe.Slice((*byte)(cb.inner.data), C.uint64_t(cb.inner.len))
	return bytes.NewReader(b)
}

func (cb GoRustBuffer) Free() {
	rustCall(func( status *C.RustCallStatus) bool {
		C.ffi_binary_options_tools_uni_rustbuffer_free(cb.inner, status)
		return false
	})
}

func (cb GoRustBuffer) ToGoBytes() []byte {
	return C.GoBytes(unsafe.Pointer(cb.inner.data), C.int(cb.inner.len))
}


func stringToRustBuffer(str string) C.RustBuffer {
	return bytesToRustBuffer([]byte(str))
}

func bytesToRustBuffer(b []byte) C.RustBuffer {
	if len(b) == 0 {
		return C.RustBuffer{}
	}
	// We can pass the pointer along here, as it is pinned
	// for the duration of this call
	foreign := C.ForeignBytes {
		len: C.int(len(b)),
		data: (*C.uchar)(unsafe.Pointer(&b[0])),
	}
	
	return rustCall(func( status *C.RustCallStatus) C.RustBuffer {
		return C.ffi_binary_options_tools_uni_rustbuffer_from_bytes(foreign, status)
	})
}


type BufLifter[GoType any] interface {
	Lift(value RustBufferI) GoType
}

type BufLowerer[GoType any] interface {
	Lower(value GoType) C.RustBuffer
}

type BufReader[GoType any] interface {
	Read(reader io.Reader) GoType
}

type BufWriter[GoType any] interface {
	Write(writer io.Writer, value GoType)
}

func LowerIntoRustBuffer[GoType any](bufWriter BufWriter[GoType], value GoType) C.RustBuffer {
	// This might be not the most efficient way but it does not require knowing allocation size
	// beforehand
	var buffer bytes.Buffer
	bufWriter.Write(&buffer, value)

	bytes, err := io.ReadAll(&buffer)
	if err != nil {
		panic(fmt.Errorf("reading written data: %w", err))
	}
	return bytesToRustBuffer(bytes)
}

func LiftFromRustBuffer[GoType any](bufReader BufReader[GoType], rbuf RustBufferI) GoType {
	defer rbuf.Free()
	reader := rbuf.AsReader()
	item := bufReader.Read(reader)
	if reader.Len() > 0 {
		// TODO: Remove this
		leftover, _ := io.ReadAll(reader)
		panic(fmt.Errorf("Junk remaining in buffer after lifting: %s", string(leftover)))
	}
	return item
}



func rustCallWithError[E any, U any](converter BufReader[*E], callback func(*C.RustCallStatus) U) (U, *E) {
	var status C.RustCallStatus
	returnValue := callback(&status)
	err := checkCallStatus(converter, status)
	return returnValue, err
}

func checkCallStatus[E any](converter BufReader[*E], status C.RustCallStatus) *E {
	switch status.code {
	case 0:
		return nil
	case 1:
		return LiftFromRustBuffer(converter, GoRustBuffer { inner: status.errorBuf })
	case 2:
		// when the rust code sees a panic, it tries to construct a rustBuffer
		// with the message.  but if that code panics, then it just sends back
		// an empty buffer.
		if status.errorBuf.len > 0 {
			panic(fmt.Errorf("%s", FfiConverterStringINSTANCE.Lift(GoRustBuffer { inner: status.errorBuf })))
		} else {
			panic(fmt.Errorf("Rust panicked while handling Rust panic"))
		}
	default:
		panic(fmt.Errorf("unknown status code: %d", status.code))
	}
}

func checkCallStatusUnknown(status C.RustCallStatus) error {
	switch status.code {
	case 0:
		return nil
	case 1:
		panic(fmt.Errorf("function not returning an error returned an error"))
	case 2:
		// when the rust code sees a panic, it tries to construct a C.RustBuffer
		// with the message.  but if that code panics, then it just sends back
		// an empty buffer.
		if status.errorBuf.len > 0 {
			panic(fmt.Errorf("%s", FfiConverterStringINSTANCE.Lift(GoRustBuffer {
				inner: status.errorBuf,
			})))
		} else {
			panic(fmt.Errorf("Rust panicked while handling Rust panic"))
		}
	default:
		return fmt.Errorf("unknown status code: %d", status.code)
	}
}

func rustCall[U any](callback func(*C.RustCallStatus) U) U {
	returnValue, err := rustCallWithError[error](nil, callback)
	if err != nil {
		panic(err)
	}
	return returnValue
}

type NativeError interface {
	AsError() error
}


func writeInt8(writer io.Writer, value int8) {
	if err := binary.Write(writer, binary.BigEndian, value); err != nil {
		panic(err)
	}
}

func writeUint8(writer io.Writer, value uint8) {
	if err := binary.Write(writer, binary.BigEndian, value); err != nil {
		panic(err)
	}
}

func writeInt16(writer io.Writer, value int16) {
	if err := binary.Write(writer, binary.BigEndian, value); err != nil {
		panic(err)
	}
}

func writeUint16(writer io.Writer, value uint16) {
	if err := binary.Write(writer, binary.BigEndian, value); err != nil {
		panic(err)
	}
}

func writeInt32(writer io.Writer, value int32) {
	if err := binary.Write(writer, binary.BigEndian, value); err != nil {
		panic(err)
	}
}

func writeUint32(writer io.Writer, value uint32) {
	if err := binary.Write(writer, binary.BigEndian, value); err != nil {
		panic(err)
	}
}

func writeInt64(writer io.Writer, value int64) {
	if err := binary.Write(writer, binary.BigEndian, value); err != nil {
		panic(err)
	}
}

func writeUint64(writer io.Writer, value uint64) {
	if err := binary.Write(writer, binary.BigEndian, value); err != nil {
		panic(err)
	}
}

func writeFloat32(writer io.Writer, value float32) {
	if err := binary.Write(writer, binary.BigEndian, value); err != nil {
		panic(err)
	}
}

func writeFloat64(writer io.Writer, value float64) {
	if err := binary.Write(writer, binary.BigEndian, value); err != nil {
		panic(err)
	}
}


func readInt8(reader io.Reader) int8 {
	var result int8
	if err := binary.Read(reader, binary.BigEndian, &result); err != nil {
		panic(err)
	}
	return result
}

func readUint8(reader io.Reader) uint8 {
	var result uint8
	if err := binary.Read(reader, binary.BigEndian, &result); err != nil {
		panic(err)
	}
	return result
}

func readInt16(reader io.Reader) int16 {
	var result int16
	if err := binary.Read(reader, binary.BigEndian, &result); err != nil {
		panic(err)
	}
	return result
}

func readUint16(reader io.Reader) uint16 {
	var result uint16
	if err := binary.Read(reader, binary.BigEndian, &result); err != nil {
		panic(err)
	}
	return result
}

func readInt32(reader io.Reader) int32 {
	var result int32
	if err := binary.Read(reader, binary.BigEndian, &result); err != nil {
		panic(err)
	}
	return result
}

func readUint32(reader io.Reader) uint32 {
	var result uint32
	if err := binary.Read(reader, binary.BigEndian, &result); err != nil {
		panic(err)
	}
	return result
}

func readInt64(reader io.Reader) int64 {
	var result int64
	if err := binary.Read(reader, binary.BigEndian, &result); err != nil {
		panic(err)
	}
	return result
}

func readUint64(reader io.Reader) uint64 {
	var result uint64
	if err := binary.Read(reader, binary.BigEndian, &result); err != nil {
		panic(err)
	}
	return result
}

func readFloat32(reader io.Reader) float32 {
	var result float32
	if err := binary.Read(reader, binary.BigEndian, &result); err != nil {
		panic(err)
	}
	return result
}

func readFloat64(reader io.Reader) float64 {
	var result float64
	if err := binary.Read(reader, binary.BigEndian, &result); err != nil {
		panic(err)
	}
	return result
}

func init() {
        
        uniffiCheckChecksums()
}


func uniffiCheckChecksums() {
	// Get the bindings contract version from our ComponentInterface
	bindingsContractVersion := 26
	// Get the scaffolding contract version by calling the into the dylib
	scaffoldingContractVersion := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint32_t {
		return C.ffi_binary_options_tools_uni_uniffi_contract_version()
	})
	if bindingsContractVersion != int(scaffoldingContractVersion) {
		// If this happens try cleaning and rebuilding your project
		panic("binary_options_tools_uni: UniFFI contract version mismatch")
	}
	{
	checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
		return C.uniffi_binary_options_tools_uni_checksum_method_pocketoption_assets()
	})
	if checksum != 48493 {
		// If this happens try cleaning and rebuilding your project
		panic("binary_options_tools_uni: uniffi_binary_options_tools_uni_checksum_method_pocketoption_assets: UniFFI API checksum mismatch")
	}
	}
	{
	checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
		return C.uniffi_binary_options_tools_uni_checksum_method_pocketoption_balance()
	})
	if checksum != 26020 {
		// If this happens try cleaning and rebuilding your project
		panic("binary_options_tools_uni: uniffi_binary_options_tools_uni_checksum_method_pocketoption_balance: UniFFI API checksum mismatch")
	}
	}
	{
	checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
		return C.uniffi_binary_options_tools_uni_checksum_method_pocketoption_buy()
	})
	if checksum != 63032 {
		// If this happens try cleaning and rebuilding your project
		panic("binary_options_tools_uni: uniffi_binary_options_tools_uni_checksum_method_pocketoption_buy: UniFFI API checksum mismatch")
	}
	}
	{
	checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
		return C.uniffi_binary_options_tools_uni_checksum_method_pocketoption_clear_closed_deals()
	})
	if checksum != 9178 {
		// If this happens try cleaning and rebuilding your project
		panic("binary_options_tools_uni: uniffi_binary_options_tools_uni_checksum_method_pocketoption_clear_closed_deals: UniFFI API checksum mismatch")
	}
	}
	{
	checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
		return C.uniffi_binary_options_tools_uni_checksum_method_pocketoption_get_candles()
	})
	if checksum != 23490 {
		// If this happens try cleaning and rebuilding your project
		panic("binary_options_tools_uni: uniffi_binary_options_tools_uni_checksum_method_pocketoption_get_candles: UniFFI API checksum mismatch")
	}
	}
	{
	checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
		return C.uniffi_binary_options_tools_uni_checksum_method_pocketoption_get_candles_advanced()
	})
	if checksum != 27509 {
		// If this happens try cleaning and rebuilding your project
		panic("binary_options_tools_uni: uniffi_binary_options_tools_uni_checksum_method_pocketoption_get_candles_advanced: UniFFI API checksum mismatch")
	}
	}
	{
	checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
		return C.uniffi_binary_options_tools_uni_checksum_method_pocketoption_get_closed_deals()
	})
	if checksum != 47785 {
		// If this happens try cleaning and rebuilding your project
		panic("binary_options_tools_uni: uniffi_binary_options_tools_uni_checksum_method_pocketoption_get_closed_deals: UniFFI API checksum mismatch")
	}
	}
	{
	checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
		return C.uniffi_binary_options_tools_uni_checksum_method_pocketoption_get_opened_deals()
	})
	if checksum != 27985 {
		// If this happens try cleaning and rebuilding your project
		panic("binary_options_tools_uni: uniffi_binary_options_tools_uni_checksum_method_pocketoption_get_opened_deals: UniFFI API checksum mismatch")
	}
	}
	{
	checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
		return C.uniffi_binary_options_tools_uni_checksum_method_pocketoption_history()
	})
	if checksum != 27093 {
		// If this happens try cleaning and rebuilding your project
		panic("binary_options_tools_uni: uniffi_binary_options_tools_uni_checksum_method_pocketoption_history: UniFFI API checksum mismatch")
	}
	}
	{
	checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
		return C.uniffi_binary_options_tools_uni_checksum_method_pocketoption_is_demo()
	})
	if checksum != 19411 {
		// If this happens try cleaning and rebuilding your project
		panic("binary_options_tools_uni: uniffi_binary_options_tools_uni_checksum_method_pocketoption_is_demo: UniFFI API checksum mismatch")
	}
	}
	{
	checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
		return C.uniffi_binary_options_tools_uni_checksum_method_pocketoption_reconnect()
	})
	if checksum != 9220 {
		// If this happens try cleaning and rebuilding your project
		panic("binary_options_tools_uni: uniffi_binary_options_tools_uni_checksum_method_pocketoption_reconnect: UniFFI API checksum mismatch")
	}
	}
	{
	checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
		return C.uniffi_binary_options_tools_uni_checksum_method_pocketoption_result()
	})
	if checksum != 594 {
		// If this happens try cleaning and rebuilding your project
		panic("binary_options_tools_uni: uniffi_binary_options_tools_uni_checksum_method_pocketoption_result: UniFFI API checksum mismatch")
	}
	}
	{
	checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
		return C.uniffi_binary_options_tools_uni_checksum_method_pocketoption_result_with_timeout()
	})
	if checksum != 56468 {
		// If this happens try cleaning and rebuilding your project
		panic("binary_options_tools_uni: uniffi_binary_options_tools_uni_checksum_method_pocketoption_result_with_timeout: UniFFI API checksum mismatch")
	}
	}
	{
	checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
		return C.uniffi_binary_options_tools_uni_checksum_method_pocketoption_sell()
	})
	if checksum != 61157 {
		// If this happens try cleaning and rebuilding your project
		panic("binary_options_tools_uni: uniffi_binary_options_tools_uni_checksum_method_pocketoption_sell: UniFFI API checksum mismatch")
	}
	}
	{
	checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
		return C.uniffi_binary_options_tools_uni_checksum_method_pocketoption_server_time()
	})
	if checksum != 10589 {
		// If this happens try cleaning and rebuilding your project
		panic("binary_options_tools_uni: uniffi_binary_options_tools_uni_checksum_method_pocketoption_server_time: UniFFI API checksum mismatch")
	}
	}
	{
	checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
		return C.uniffi_binary_options_tools_uni_checksum_method_pocketoption_shutdown()
	})
	if checksum != 51452 {
		// If this happens try cleaning and rebuilding your project
		panic("binary_options_tools_uni: uniffi_binary_options_tools_uni_checksum_method_pocketoption_shutdown: UniFFI API checksum mismatch")
	}
	}
	{
	checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
		return C.uniffi_binary_options_tools_uni_checksum_method_pocketoption_subscribe()
	})
	if checksum != 23382 {
		// If this happens try cleaning and rebuilding your project
		panic("binary_options_tools_uni: uniffi_binary_options_tools_uni_checksum_method_pocketoption_subscribe: UniFFI API checksum mismatch")
	}
	}
	{
	checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
		return C.uniffi_binary_options_tools_uni_checksum_method_pocketoption_trade()
	})
	if checksum != 14619 {
		// If this happens try cleaning and rebuilding your project
		panic("binary_options_tools_uni: uniffi_binary_options_tools_uni_checksum_method_pocketoption_trade: UniFFI API checksum mismatch")
	}
	}
	{
	checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
		return C.uniffi_binary_options_tools_uni_checksum_method_pocketoption_unsubscribe()
	})
	if checksum != 29837 {
		// If this happens try cleaning and rebuilding your project
		panic("binary_options_tools_uni: uniffi_binary_options_tools_uni_checksum_method_pocketoption_unsubscribe: UniFFI API checksum mismatch")
	}
	}
	{
	checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
		return C.uniffi_binary_options_tools_uni_checksum_method_subscriptionstream_next()
	})
	if checksum != 13448 {
		// If this happens try cleaning and rebuilding your project
		panic("binary_options_tools_uni: uniffi_binary_options_tools_uni_checksum_method_subscriptionstream_next: UniFFI API checksum mismatch")
	}
	}
	{
	checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
		return C.uniffi_binary_options_tools_uni_checksum_constructor_pocketoption_new()
	})
	if checksum != 31315 {
		// If this happens try cleaning and rebuilding your project
		panic("binary_options_tools_uni: uniffi_binary_options_tools_uni_checksum_constructor_pocketoption_new: UniFFI API checksum mismatch")
	}
	}
	{
	checksum := rustCall(func(_uniffiStatus *C.RustCallStatus) C.uint16_t {
		return C.uniffi_binary_options_tools_uni_checksum_constructor_pocketoption_new_with_url()
	})
	if checksum != 40992 {
		// If this happens try cleaning and rebuilding your project
		panic("binary_options_tools_uni: uniffi_binary_options_tools_uni_checksum_constructor_pocketoption_new_with_url: UniFFI API checksum mismatch")
	}
	}
}




type FfiConverterUint32 struct{}

var FfiConverterUint32INSTANCE = FfiConverterUint32{}

func (FfiConverterUint32) Lower(value uint32) C.uint32_t {
	return C.uint32_t(value)
}

func (FfiConverterUint32) Write(writer io.Writer, value uint32) {
	writeUint32(writer, value)
}

func (FfiConverterUint32) Lift(value C.uint32_t) uint32 {
	return uint32(value)
}

func (FfiConverterUint32) Read(reader io.Reader) uint32 {
	return readUint32(reader)
}

type FfiDestroyerUint32 struct {}

func (FfiDestroyerUint32) Destroy(_ uint32) {}


type FfiConverterInt32 struct{}

var FfiConverterInt32INSTANCE = FfiConverterInt32{}

func (FfiConverterInt32) Lower(value int32) C.int32_t {
	return C.int32_t(value)
}

func (FfiConverterInt32) Write(writer io.Writer, value int32) {
	writeInt32(writer, value)
}

func (FfiConverterInt32) Lift(value C.int32_t) int32 {
	return int32(value)
}

func (FfiConverterInt32) Read(reader io.Reader) int32 {
	return readInt32(reader)
}

type FfiDestroyerInt32 struct {}

func (FfiDestroyerInt32) Destroy(_ int32) {}


type FfiConverterUint64 struct{}

var FfiConverterUint64INSTANCE = FfiConverterUint64{}

func (FfiConverterUint64) Lower(value uint64) C.uint64_t {
	return C.uint64_t(value)
}

func (FfiConverterUint64) Write(writer io.Writer, value uint64) {
	writeUint64(writer, value)
}

func (FfiConverterUint64) Lift(value C.uint64_t) uint64 {
	return uint64(value)
}

func (FfiConverterUint64) Read(reader io.Reader) uint64 {
	return readUint64(reader)
}

type FfiDestroyerUint64 struct {}

func (FfiDestroyerUint64) Destroy(_ uint64) {}


type FfiConverterInt64 struct{}

var FfiConverterInt64INSTANCE = FfiConverterInt64{}

func (FfiConverterInt64) Lower(value int64) C.int64_t {
	return C.int64_t(value)
}

func (FfiConverterInt64) Write(writer io.Writer, value int64) {
	writeInt64(writer, value)
}

func (FfiConverterInt64) Lift(value C.int64_t) int64 {
	return int64(value)
}

func (FfiConverterInt64) Read(reader io.Reader) int64 {
	return readInt64(reader)
}

type FfiDestroyerInt64 struct {}

func (FfiDestroyerInt64) Destroy(_ int64) {}


type FfiConverterFloat64 struct{}

var FfiConverterFloat64INSTANCE = FfiConverterFloat64{}

func (FfiConverterFloat64) Lower(value float64) C.double {
	return C.double(value)
}

func (FfiConverterFloat64) Write(writer io.Writer, value float64) {
	writeFloat64(writer, value)
}

func (FfiConverterFloat64) Lift(value C.double) float64 {
	return float64(value)
}

func (FfiConverterFloat64) Read(reader io.Reader) float64 {
	return readFloat64(reader)
}

type FfiDestroyerFloat64 struct {}

func (FfiDestroyerFloat64) Destroy(_ float64) {}


type FfiConverterBool struct{}

var FfiConverterBoolINSTANCE = FfiConverterBool{}

func (FfiConverterBool) Lower(value bool) C.int8_t {
	if value {
		return C.int8_t(1)
	}
	return C.int8_t(0)
}

func (FfiConverterBool) Write(writer io.Writer, value bool) {
	if value {
		writeInt8(writer, 1)
	} else {
		writeInt8(writer, 0)
	}
}

func (FfiConverterBool) Lift(value C.int8_t) bool {
	return value != 0
}

func (FfiConverterBool) Read(reader io.Reader) bool {
	return readInt8(reader) != 0
}

type FfiDestroyerBool struct {}

func (FfiDestroyerBool) Destroy(_ bool) {}


type FfiConverterString struct{}

var FfiConverterStringINSTANCE = FfiConverterString{}

func (FfiConverterString) Lift(rb RustBufferI) string {
	defer rb.Free()
	reader := rb.AsReader()
	b, err := io.ReadAll(reader)
	if err != nil {
		panic(fmt.Errorf("reading reader: %w", err))
	}
	return string(b)
}

func (FfiConverterString) Read(reader io.Reader) string {
	length := readInt32(reader)
	buffer := make([]byte, length)
	read_length, err := reader.Read(buffer)
	if err != nil && err != io.EOF {
		panic(err)
	}
	if read_length != int(length) {
		panic(fmt.Errorf("bad read length when reading string, expected %d, read %d", length, read_length))
	}
	return string(buffer)
}

func (FfiConverterString) Lower(value string) C.RustBuffer {
	return stringToRustBuffer(value)
}

func (FfiConverterString) Write(writer io.Writer, value string) {
	if len(value) > math.MaxInt32 {
		panic("String is too large to fit into Int32")
	}

	writeInt32(writer, int32(len(value)))
	write_length, err := io.WriteString(writer, value)
	if err != nil {
		panic(err)
	}
	if write_length != len(value) {
		panic(fmt.Errorf("bad write length when writing string, expected %d, written %d", len(value), write_length))
	}
}

type FfiDestroyerString struct {}

func (FfiDestroyerString) Destroy(_ string) {}



// Below is an implementation of synchronization requirements outlined in the link.
// https://github.com/mozilla/uniffi-rs/blob/0dc031132d9493ca812c3af6e7dd60ad2ea95bf0/uniffi_bindgen/src/bindings/kotlin/templates/ObjectRuntime.kt#L31

type FfiObject struct {
	pointer unsafe.Pointer
	callCounter atomic.Int64
	cloneFunction func(unsafe.Pointer, *C.RustCallStatus) unsafe.Pointer
	freeFunction func(unsafe.Pointer, *C.RustCallStatus)
	destroyed atomic.Bool
}

func newFfiObject(
	pointer unsafe.Pointer, 
	cloneFunction func(unsafe.Pointer, *C.RustCallStatus) unsafe.Pointer, 
	freeFunction func(unsafe.Pointer, *C.RustCallStatus),
) FfiObject {
	return FfiObject {
		pointer: pointer,
		cloneFunction: cloneFunction, 
		freeFunction: freeFunction,
	}
}

func (ffiObject *FfiObject)incrementPointer(debugName string) unsafe.Pointer {
	for {
		counter := ffiObject.callCounter.Load()
		if counter <= -1 {
			panic(fmt.Errorf("%v object has already been destroyed", debugName))
		}
		if counter == math.MaxInt64 {
			panic(fmt.Errorf("%v object call counter would overflow", debugName))
		}
		if ffiObject.callCounter.CompareAndSwap(counter, counter + 1) {
			break
		}
	}

	return rustCall(func(status *C.RustCallStatus) unsafe.Pointer {
		return ffiObject.cloneFunction(ffiObject.pointer, status)
	})
}

func (ffiObject *FfiObject)decrementPointer() {
	if ffiObject.callCounter.Add(-1) == -1 {
		ffiObject.freeRustArcPtr()
	}
}

func (ffiObject *FfiObject)destroy() {
	if ffiObject.destroyed.CompareAndSwap(false, true) {
		if ffiObject.callCounter.Add(-1) == -1 {
			ffiObject.freeRustArcPtr()
		}
	}
}

func (ffiObject *FfiObject)freeRustArcPtr() {
	rustCall(func(status *C.RustCallStatus) int32 {
		ffiObject.freeFunction(ffiObject.pointer, status)
		return 0
	})
}
// The main client for interacting with the PocketOption platform.
//
// This object provides all the functionality needed to connect to PocketOption,
// place trades, get account information, and subscribe to market data.
//
// It is the primary entry point for using this library.
//
// # Rationale
//
// This struct wraps the underlying `binary_options_tools::pocketoption::PocketOption` client,
// exposing its functionality in a way that is compatible with UniFFI for creating
// multi-language bindings.
type PocketOptionInterface interface {
	// Gets the list of available assets for trading.
	//
	// # Returns
	//
	// A list of `Asset` objects, or `None` if the assets have not been loaded yet.
	Assets() *[]Asset
	// Gets the current balance of the account.
	//
	// This method retrieves the current trading balance from the client's state.
	//
	// # Returns
	//
	// The current balance as a floating-point number.
	Balance() float64
	// Places a "Call" (buy) trade.
	//
	// This is a convenience method that calls `trade` with `Action.Call`.
	Buy(asset string, time uint32, amount float64) (Deal, error)
	// Clears the list of closed deals from the client's state.
	ClearClosedDeals() 
	// Gets historical candle data for a specific asset.
	GetCandles(asset string, period int64, offset int64) ([]Candle, error)
	// Gets historical candle data for a specific asset with advanced parameters.
	GetCandlesAdvanced(asset string, period int64, time int64, offset int64) ([]Candle, error)
	// Gets the list of currently closed deals.
	GetClosedDeals() []Deal
	// Gets the list of currently opened deals.
	GetOpenedDeals() []Deal
	// Gets historical candle data for a specific asset and period.
	History(asset string, period uint32) ([]Candle, error)
	// Checks if the current session is a demo account.
	//
	// # Returns
	//
	// `true` if the account is a demo account, `false` otherwise.
	IsDemo() bool
	// Disconnects and reconnects the client.
	Reconnect() error
	// Checks the result of a trade by its ID.
	//
	// # Arguments
	//
	// * `id` - The ID of the trade to check (as a string).
	//
	// # Returns
	//
	// A `Deal` object representing the completed trade.
	Result(id string) (Deal, error)
	// Checks the result of a trade by its ID with a timeout.
	//
	// # Arguments
	//
	// * `id` - The ID of the trade to check (as a string).
	// * `timeout_secs` - The maximum time to wait for the result in seconds.
	//
	// # Returns
	//
	// A `Deal` object representing the completed trade.
	ResultWithTimeout(id string, timeoutSecs uint64) (Deal, error)
	// Places a "Put" (sell) trade.
	//
	// This is a convenience method that calls `trade` with `Action.Put`.
	Sell(asset string, time uint32, amount float64) (Deal, error)
	// Gets the current server time as a Unix timestamp.
	ServerTime() int64
	// Shuts down the client and stops all background tasks.
	//
	// This method should be called when you are finished with the client
	// to ensure a graceful shutdown.
	Shutdown() error
	// Subscribes to real-time candle data for a specific asset.
	//
	// # Arguments
	//
	// * `asset` - The symbol of the asset to subscribe to.
	// * `duration_secs` - The duration of each candle in seconds.
	//
	// # Returns
	//
	// A `SubscriptionStream` object that can be used to receive candle data.
	Subscribe(asset string, durationSecs uint64) (*SubscriptionStream, error)
	// Places a trade.
	//
	// This is the core method for executing trades.
	//
	// # Arguments
	//
	// * `asset` - The symbol of the asset to trade (e.g., "EURUSD_otc").
	// * `action` - The direction of the trade (`Action.Call` or `Action.Put`).
	// * `time` - The duration of the trade in seconds.
	// * `amount` - The amount to trade.
	//
	// # Returns
	//
	// A `Deal` object representing the completed trade.
	Trade(asset string, action Action, time uint32, amount float64) (Deal, error)
	// Unsubscribes from real-time candle data for a specific asset.
	Unsubscribe(asset string) error
}
// The main client for interacting with the PocketOption platform.
//
// This object provides all the functionality needed to connect to PocketOption,
// place trades, get account information, and subscribe to market data.
//
// It is the primary entry point for using this library.
//
// # Rationale
//
// This struct wraps the underlying `binary_options_tools::pocketoption::PocketOption` client,
// exposing its functionality in a way that is compatible with UniFFI for creating
// multi-language bindings.
type PocketOption struct {
	ffiObject FfiObject
}
// Creates a new instance of the PocketOption client.
//
// This is the primary constructor for the client. It requires a session ID (ssid)
// to authenticate with the PocketOption servers.
//
// # Arguments
//
// * `ssid` - The session ID for your PocketOption account.
//
// # Examples
//
// ## Python
// ```python
// import asyncio
// from binaryoptionstoolsuni import PocketOption
//
// async def main():
// ssid = "YOUR_SESSION_ID"
// api = await PocketOption.new(ssid)
// balance = await api.balance()
// print(f"Balance: {balance}")
//
// asyncio.run(main())
// ```
func NewPocketOption(ssid string) (*PocketOption, error) {
	 res, err :=uniffiRustCallAsync[UniError](
        FfiConverterUniErrorINSTANCE,
		// completeFn
		func(handle C.uint64_t, status *C.RustCallStatus) unsafe.Pointer {
			res := C.ffi_binary_options_tools_uni_rust_future_complete_pointer(handle, status)
			return res
		},
		// liftFn
		func(ffi unsafe.Pointer) *PocketOption {
			return FfiConverterPocketOptionINSTANCE.Lift(ffi)
		},
		C.uniffi_binary_options_tools_uni_fn_constructor_pocketoption_new(FfiConverterStringINSTANCE.Lower(ssid)),
		// pollFn
		func (handle C.uint64_t, continuation C.UniffiRustFutureContinuationCallback, data C.uint64_t) {
			C.ffi_binary_options_tools_uni_rust_future_poll_pointer(handle, continuation, data)
		},
		// freeFn
		func (handle C.uint64_t) {
			C.ffi_binary_options_tools_uni_rust_future_free_pointer(handle)
		},
	)

	return res, err 
}


// Creates a new instance of the PocketOption client with a custom WebSocket URL.
//
// This constructor is useful for connecting to different PocketOption servers,
// for example, in different regions.
//
// # Arguments
//
// * `ssid` - The session ID for your PocketOption account.
// * `url` - The custom WebSocket URL to connect to.
func PocketOptionNewWithUrl(ssid string, url string) (*PocketOption, error) {
	 res, err :=uniffiRustCallAsync[UniError](
        FfiConverterUniErrorINSTANCE,
		// completeFn
		func(handle C.uint64_t, status *C.RustCallStatus) unsafe.Pointer {
			res := C.ffi_binary_options_tools_uni_rust_future_complete_pointer(handle, status)
			return res
		},
		// liftFn
		func(ffi unsafe.Pointer) *PocketOption {
			return FfiConverterPocketOptionINSTANCE.Lift(ffi)
		},
		C.uniffi_binary_options_tools_uni_fn_constructor_pocketoption_new_with_url(FfiConverterStringINSTANCE.Lower(ssid), FfiConverterStringINSTANCE.Lower(url)),
		// pollFn
		func (handle C.uint64_t, continuation C.UniffiRustFutureContinuationCallback, data C.uint64_t) {
			C.ffi_binary_options_tools_uni_rust_future_poll_pointer(handle, continuation, data)
		},
		// freeFn
		func (handle C.uint64_t) {
			C.ffi_binary_options_tools_uni_rust_future_free_pointer(handle)
		},
	)

	return res, err 
}



// Gets the list of available assets for trading.
//
// # Returns
//
// A list of `Asset` objects, or `None` if the assets have not been loaded yet.
func (_self *PocketOption) Assets() *[]Asset {
	_pointer := _self.ffiObject.incrementPointer("*PocketOption")
	defer _self.ffiObject.decrementPointer()
	 res, _ :=uniffiRustCallAsync[error](
        nil,
		// completeFn
		func(handle C.uint64_t, status *C.RustCallStatus) RustBufferI {
			res := C.ffi_binary_options_tools_uni_rust_future_complete_rust_buffer(handle, status)
			return GoRustBuffer {
		inner: res,
	}
		},
		// liftFn
		func(ffi RustBufferI) *[]Asset {
			return FfiConverterOptionalSequenceAssetINSTANCE.Lift(ffi)
		},
		C.uniffi_binary_options_tools_uni_fn_method_pocketoption_assets(
		_pointer,),
		// pollFn
		func (handle C.uint64_t, continuation C.UniffiRustFutureContinuationCallback, data C.uint64_t) {
			C.ffi_binary_options_tools_uni_rust_future_poll_rust_buffer(handle, continuation, data)
		},
		// freeFn
		func (handle C.uint64_t) {
			C.ffi_binary_options_tools_uni_rust_future_free_rust_buffer(handle)
		},
	)

	return res 
}

// Gets the current balance of the account.
//
// This method retrieves the current trading balance from the client's state.
//
// # Returns
//
// The current balance as a floating-point number.
func (_self *PocketOption) Balance() float64 {
	_pointer := _self.ffiObject.incrementPointer("*PocketOption")
	defer _self.ffiObject.decrementPointer()
	 res, _ :=uniffiRustCallAsync[error](
        nil,
		// completeFn
		func(handle C.uint64_t, status *C.RustCallStatus) C.double {
			res := C.ffi_binary_options_tools_uni_rust_future_complete_f64(handle, status)
			return res
		},
		// liftFn
		func(ffi C.double) float64 {
			return FfiConverterFloat64INSTANCE.Lift(ffi)
		},
		C.uniffi_binary_options_tools_uni_fn_method_pocketoption_balance(
		_pointer,),
		// pollFn
		func (handle C.uint64_t, continuation C.UniffiRustFutureContinuationCallback, data C.uint64_t) {
			C.ffi_binary_options_tools_uni_rust_future_poll_f64(handle, continuation, data)
		},
		// freeFn
		func (handle C.uint64_t) {
			C.ffi_binary_options_tools_uni_rust_future_free_f64(handle)
		},
	)

	return res 
}

// Places a "Call" (buy) trade.
//
// This is a convenience method that calls `trade` with `Action.Call`.
func (_self *PocketOption) Buy(asset string, time uint32, amount float64) (Deal, error) {
	_pointer := _self.ffiObject.incrementPointer("*PocketOption")
	defer _self.ffiObject.decrementPointer()
	 res, err :=uniffiRustCallAsync[UniError](
        FfiConverterUniErrorINSTANCE,
		// completeFn
		func(handle C.uint64_t, status *C.RustCallStatus) RustBufferI {
			res := C.ffi_binary_options_tools_uni_rust_future_complete_rust_buffer(handle, status)
			return GoRustBuffer {
		inner: res,
	}
		},
		// liftFn
		func(ffi RustBufferI) Deal {
			return FfiConverterDealINSTANCE.Lift(ffi)
		},
		C.uniffi_binary_options_tools_uni_fn_method_pocketoption_buy(
		_pointer,FfiConverterStringINSTANCE.Lower(asset), FfiConverterUint32INSTANCE.Lower(time), FfiConverterFloat64INSTANCE.Lower(amount)),
		// pollFn
		func (handle C.uint64_t, continuation C.UniffiRustFutureContinuationCallback, data C.uint64_t) {
			C.ffi_binary_options_tools_uni_rust_future_poll_rust_buffer(handle, continuation, data)
		},
		// freeFn
		func (handle C.uint64_t) {
			C.ffi_binary_options_tools_uni_rust_future_free_rust_buffer(handle)
		},
	)

	return res, err 
}

// Clears the list of closed deals from the client's state.
func (_self *PocketOption) ClearClosedDeals()  {
	_pointer := _self.ffiObject.incrementPointer("*PocketOption")
	defer _self.ffiObject.decrementPointer()
	uniffiRustCallAsync[error](
        nil,
		// completeFn
		func(handle C.uint64_t, status *C.RustCallStatus) struct{} {
			C.ffi_binary_options_tools_uni_rust_future_complete_void(handle, status)
			return struct{}{}
		},
		// liftFn
		func(_ struct{}) struct{} { return struct{}{} },
		C.uniffi_binary_options_tools_uni_fn_method_pocketoption_clear_closed_deals(
		_pointer,),
		// pollFn
		func (handle C.uint64_t, continuation C.UniffiRustFutureContinuationCallback, data C.uint64_t) {
			C.ffi_binary_options_tools_uni_rust_future_poll_void(handle, continuation, data)
		},
		// freeFn
		func (handle C.uint64_t) {
			C.ffi_binary_options_tools_uni_rust_future_free_void(handle)
		},
	)

	
}

// Gets historical candle data for a specific asset.
func (_self *PocketOption) GetCandles(asset string, period int64, offset int64) ([]Candle, error) {
	_pointer := _self.ffiObject.incrementPointer("*PocketOption")
	defer _self.ffiObject.decrementPointer()
	 res, err :=uniffiRustCallAsync[UniError](
        FfiConverterUniErrorINSTANCE,
		// completeFn
		func(handle C.uint64_t, status *C.RustCallStatus) RustBufferI {
			res := C.ffi_binary_options_tools_uni_rust_future_complete_rust_buffer(handle, status)
			return GoRustBuffer {
		inner: res,
	}
		},
		// liftFn
		func(ffi RustBufferI) []Candle {
			return FfiConverterSequenceCandleINSTANCE.Lift(ffi)
		},
		C.uniffi_binary_options_tools_uni_fn_method_pocketoption_get_candles(
		_pointer,FfiConverterStringINSTANCE.Lower(asset), FfiConverterInt64INSTANCE.Lower(period), FfiConverterInt64INSTANCE.Lower(offset)),
		// pollFn
		func (handle C.uint64_t, continuation C.UniffiRustFutureContinuationCallback, data C.uint64_t) {
			C.ffi_binary_options_tools_uni_rust_future_poll_rust_buffer(handle, continuation, data)
		},
		// freeFn
		func (handle C.uint64_t) {
			C.ffi_binary_options_tools_uni_rust_future_free_rust_buffer(handle)
		},
	)

	return res, err 
}

// Gets historical candle data for a specific asset with advanced parameters.
func (_self *PocketOption) GetCandlesAdvanced(asset string, period int64, time int64, offset int64) ([]Candle, error) {
	_pointer := _self.ffiObject.incrementPointer("*PocketOption")
	defer _self.ffiObject.decrementPointer()
	 res, err :=uniffiRustCallAsync[UniError](
        FfiConverterUniErrorINSTANCE,
		// completeFn
		func(handle C.uint64_t, status *C.RustCallStatus) RustBufferI {
			res := C.ffi_binary_options_tools_uni_rust_future_complete_rust_buffer(handle, status)
			return GoRustBuffer {
		inner: res,
	}
		},
		// liftFn
		func(ffi RustBufferI) []Candle {
			return FfiConverterSequenceCandleINSTANCE.Lift(ffi)
		},
		C.uniffi_binary_options_tools_uni_fn_method_pocketoption_get_candles_advanced(
		_pointer,FfiConverterStringINSTANCE.Lower(asset), FfiConverterInt64INSTANCE.Lower(period), FfiConverterInt64INSTANCE.Lower(time), FfiConverterInt64INSTANCE.Lower(offset)),
		// pollFn
		func (handle C.uint64_t, continuation C.UniffiRustFutureContinuationCallback, data C.uint64_t) {
			C.ffi_binary_options_tools_uni_rust_future_poll_rust_buffer(handle, continuation, data)
		},
		// freeFn
		func (handle C.uint64_t) {
			C.ffi_binary_options_tools_uni_rust_future_free_rust_buffer(handle)
		},
	)

	return res, err 
}

// Gets the list of currently closed deals.
func (_self *PocketOption) GetClosedDeals() []Deal {
	_pointer := _self.ffiObject.incrementPointer("*PocketOption")
	defer _self.ffiObject.decrementPointer()
	 res, _ :=uniffiRustCallAsync[error](
        nil,
		// completeFn
		func(handle C.uint64_t, status *C.RustCallStatus) RustBufferI {
			res := C.ffi_binary_options_tools_uni_rust_future_complete_rust_buffer(handle, status)
			return GoRustBuffer {
		inner: res,
	}
		},
		// liftFn
		func(ffi RustBufferI) []Deal {
			return FfiConverterSequenceDealINSTANCE.Lift(ffi)
		},
		C.uniffi_binary_options_tools_uni_fn_method_pocketoption_get_closed_deals(
		_pointer,),
		// pollFn
		func (handle C.uint64_t, continuation C.UniffiRustFutureContinuationCallback, data C.uint64_t) {
			C.ffi_binary_options_tools_uni_rust_future_poll_rust_buffer(handle, continuation, data)
		},
		// freeFn
		func (handle C.uint64_t) {
			C.ffi_binary_options_tools_uni_rust_future_free_rust_buffer(handle)
		},
	)

	return res 
}

// Gets the list of currently opened deals.
func (_self *PocketOption) GetOpenedDeals() []Deal {
	_pointer := _self.ffiObject.incrementPointer("*PocketOption")
	defer _self.ffiObject.decrementPointer()
	 res, _ :=uniffiRustCallAsync[error](
        nil,
		// completeFn
		func(handle C.uint64_t, status *C.RustCallStatus) RustBufferI {
			res := C.ffi_binary_options_tools_uni_rust_future_complete_rust_buffer(handle, status)
			return GoRustBuffer {
		inner: res,
	}
		},
		// liftFn
		func(ffi RustBufferI) []Deal {
			return FfiConverterSequenceDealINSTANCE.Lift(ffi)
		},
		C.uniffi_binary_options_tools_uni_fn_method_pocketoption_get_opened_deals(
		_pointer,),
		// pollFn
		func (handle C.uint64_t, continuation C.UniffiRustFutureContinuationCallback, data C.uint64_t) {
			C.ffi_binary_options_tools_uni_rust_future_poll_rust_buffer(handle, continuation, data)
		},
		// freeFn
		func (handle C.uint64_t) {
			C.ffi_binary_options_tools_uni_rust_future_free_rust_buffer(handle)
		},
	)

	return res 
}

// Gets historical candle data for a specific asset and period.
func (_self *PocketOption) History(asset string, period uint32) ([]Candle, error) {
	_pointer := _self.ffiObject.incrementPointer("*PocketOption")
	defer _self.ffiObject.decrementPointer()
	 res, err :=uniffiRustCallAsync[UniError](
        FfiConverterUniErrorINSTANCE,
		// completeFn
		func(handle C.uint64_t, status *C.RustCallStatus) RustBufferI {
			res := C.ffi_binary_options_tools_uni_rust_future_complete_rust_buffer(handle, status)
			return GoRustBuffer {
		inner: res,
	}
		},
		// liftFn
		func(ffi RustBufferI) []Candle {
			return FfiConverterSequenceCandleINSTANCE.Lift(ffi)
		},
		C.uniffi_binary_options_tools_uni_fn_method_pocketoption_history(
		_pointer,FfiConverterStringINSTANCE.Lower(asset), FfiConverterUint32INSTANCE.Lower(period)),
		// pollFn
		func (handle C.uint64_t, continuation C.UniffiRustFutureContinuationCallback, data C.uint64_t) {
			C.ffi_binary_options_tools_uni_rust_future_poll_rust_buffer(handle, continuation, data)
		},
		// freeFn
		func (handle C.uint64_t) {
			C.ffi_binary_options_tools_uni_rust_future_free_rust_buffer(handle)
		},
	)

	return res, err 
}

// Checks if the current session is a demo account.
//
// # Returns
//
// `true` if the account is a demo account, `false` otherwise.
func (_self *PocketOption) IsDemo() bool {
	_pointer := _self.ffiObject.incrementPointer("*PocketOption")
	defer _self.ffiObject.decrementPointer()
	return FfiConverterBoolINSTANCE.Lift(rustCall(func(_uniffiStatus *C.RustCallStatus) C.int8_t {
		return C.uniffi_binary_options_tools_uni_fn_method_pocketoption_is_demo(
		_pointer,_uniffiStatus)
	}))
}

// Disconnects and reconnects the client.
func (_self *PocketOption) Reconnect() error {
	_pointer := _self.ffiObject.incrementPointer("*PocketOption")
	defer _self.ffiObject.decrementPointer()
	 _, err :=uniffiRustCallAsync[UniError](
        FfiConverterUniErrorINSTANCE,
		// completeFn
		func(handle C.uint64_t, status *C.RustCallStatus) struct{} {
			C.ffi_binary_options_tools_uni_rust_future_complete_void(handle, status)
			return struct{}{}
		},
		// liftFn
		func(_ struct{}) struct{} { return struct{}{} },
		C.uniffi_binary_options_tools_uni_fn_method_pocketoption_reconnect(
		_pointer,),
		// pollFn
		func (handle C.uint64_t, continuation C.UniffiRustFutureContinuationCallback, data C.uint64_t) {
			C.ffi_binary_options_tools_uni_rust_future_poll_void(handle, continuation, data)
		},
		// freeFn
		func (handle C.uint64_t) {
			C.ffi_binary_options_tools_uni_rust_future_free_void(handle)
		},
	)

	return err 
}

// Checks the result of a trade by its ID.
//
// # Arguments
//
// * `id` - The ID of the trade to check (as a string).
//
// # Returns
//
// A `Deal` object representing the completed trade.
func (_self *PocketOption) Result(id string) (Deal, error) {
	_pointer := _self.ffiObject.incrementPointer("*PocketOption")
	defer _self.ffiObject.decrementPointer()
	 res, err :=uniffiRustCallAsync[UniError](
        FfiConverterUniErrorINSTANCE,
		// completeFn
		func(handle C.uint64_t, status *C.RustCallStatus) RustBufferI {
			res := C.ffi_binary_options_tools_uni_rust_future_complete_rust_buffer(handle, status)
			return GoRustBuffer {
		inner: res,
	}
		},
		// liftFn
		func(ffi RustBufferI) Deal {
			return FfiConverterDealINSTANCE.Lift(ffi)
		},
		C.uniffi_binary_options_tools_uni_fn_method_pocketoption_result(
		_pointer,FfiConverterStringINSTANCE.Lower(id)),
		// pollFn
		func (handle C.uint64_t, continuation C.UniffiRustFutureContinuationCallback, data C.uint64_t) {
			C.ffi_binary_options_tools_uni_rust_future_poll_rust_buffer(handle, continuation, data)
		},
		// freeFn
		func (handle C.uint64_t) {
			C.ffi_binary_options_tools_uni_rust_future_free_rust_buffer(handle)
		},
	)

	return res, err 
}

// Checks the result of a trade by its ID with a timeout.
//
// # Arguments
//
// * `id` - The ID of the trade to check (as a string).
// * `timeout_secs` - The maximum time to wait for the result in seconds.
//
// # Returns
//
// A `Deal` object representing the completed trade.
func (_self *PocketOption) ResultWithTimeout(id string, timeoutSecs uint64) (Deal, error) {
	_pointer := _self.ffiObject.incrementPointer("*PocketOption")
	defer _self.ffiObject.decrementPointer()
	 res, err :=uniffiRustCallAsync[UniError](
        FfiConverterUniErrorINSTANCE,
		// completeFn
		func(handle C.uint64_t, status *C.RustCallStatus) RustBufferI {
			res := C.ffi_binary_options_tools_uni_rust_future_complete_rust_buffer(handle, status)
			return GoRustBuffer {
		inner: res,
	}
		},
		// liftFn
		func(ffi RustBufferI) Deal {
			return FfiConverterDealINSTANCE.Lift(ffi)
		},
		C.uniffi_binary_options_tools_uni_fn_method_pocketoption_result_with_timeout(
		_pointer,FfiConverterStringINSTANCE.Lower(id), FfiConverterUint64INSTANCE.Lower(timeoutSecs)),
		// pollFn
		func (handle C.uint64_t, continuation C.UniffiRustFutureContinuationCallback, data C.uint64_t) {
			C.ffi_binary_options_tools_uni_rust_future_poll_rust_buffer(handle, continuation, data)
		},
		// freeFn
		func (handle C.uint64_t) {
			C.ffi_binary_options_tools_uni_rust_future_free_rust_buffer(handle)
		},
	)

	return res, err 
}

// Places a "Put" (sell) trade.
//
// This is a convenience method that calls `trade` with `Action.Put`.
func (_self *PocketOption) Sell(asset string, time uint32, amount float64) (Deal, error) {
	_pointer := _self.ffiObject.incrementPointer("*PocketOption")
	defer _self.ffiObject.decrementPointer()
	 res, err :=uniffiRustCallAsync[UniError](
        FfiConverterUniErrorINSTANCE,
		// completeFn
		func(handle C.uint64_t, status *C.RustCallStatus) RustBufferI {
			res := C.ffi_binary_options_tools_uni_rust_future_complete_rust_buffer(handle, status)
			return GoRustBuffer {
		inner: res,
	}
		},
		// liftFn
		func(ffi RustBufferI) Deal {
			return FfiConverterDealINSTANCE.Lift(ffi)
		},
		C.uniffi_binary_options_tools_uni_fn_method_pocketoption_sell(
		_pointer,FfiConverterStringINSTANCE.Lower(asset), FfiConverterUint32INSTANCE.Lower(time), FfiConverterFloat64INSTANCE.Lower(amount)),
		// pollFn
		func (handle C.uint64_t, continuation C.UniffiRustFutureContinuationCallback, data C.uint64_t) {
			C.ffi_binary_options_tools_uni_rust_future_poll_rust_buffer(handle, continuation, data)
		},
		// freeFn
		func (handle C.uint64_t) {
			C.ffi_binary_options_tools_uni_rust_future_free_rust_buffer(handle)
		},
	)

	return res, err 
}

// Gets the current server time as a Unix timestamp.
func (_self *PocketOption) ServerTime() int64 {
	_pointer := _self.ffiObject.incrementPointer("*PocketOption")
	defer _self.ffiObject.decrementPointer()
	 res, _ :=uniffiRustCallAsync[error](
        nil,
		// completeFn
		func(handle C.uint64_t, status *C.RustCallStatus) C.int64_t {
			res := C.ffi_binary_options_tools_uni_rust_future_complete_i64(handle, status)
			return res
		},
		// liftFn
		func(ffi C.int64_t) int64 {
			return FfiConverterInt64INSTANCE.Lift(ffi)
		},
		C.uniffi_binary_options_tools_uni_fn_method_pocketoption_server_time(
		_pointer,),
		// pollFn
		func (handle C.uint64_t, continuation C.UniffiRustFutureContinuationCallback, data C.uint64_t) {
			C.ffi_binary_options_tools_uni_rust_future_poll_i64(handle, continuation, data)
		},
		// freeFn
		func (handle C.uint64_t) {
			C.ffi_binary_options_tools_uni_rust_future_free_i64(handle)
		},
	)

	return res 
}

// Shuts down the client and stops all background tasks.
//
// This method should be called when you are finished with the client
// to ensure a graceful shutdown.
func (_self *PocketOption) Shutdown() error {
	_pointer := _self.ffiObject.incrementPointer("*PocketOption")
	defer _self.ffiObject.decrementPointer()
	 _, err :=uniffiRustCallAsync[UniError](
        FfiConverterUniErrorINSTANCE,
		// completeFn
		func(handle C.uint64_t, status *C.RustCallStatus) struct{} {
			C.ffi_binary_options_tools_uni_rust_future_complete_void(handle, status)
			return struct{}{}
		},
		// liftFn
		func(_ struct{}) struct{} { return struct{}{} },
		C.uniffi_binary_options_tools_uni_fn_method_pocketoption_shutdown(
		_pointer,),
		// pollFn
		func (handle C.uint64_t, continuation C.UniffiRustFutureContinuationCallback, data C.uint64_t) {
			C.ffi_binary_options_tools_uni_rust_future_poll_void(handle, continuation, data)
		},
		// freeFn
		func (handle C.uint64_t) {
			C.ffi_binary_options_tools_uni_rust_future_free_void(handle)
		},
	)

	return err 
}

// Subscribes to real-time candle data for a specific asset.
//
// # Arguments
//
// * `asset` - The symbol of the asset to subscribe to.
// * `duration_secs` - The duration of each candle in seconds.
//
// # Returns
//
// A `SubscriptionStream` object that can be used to receive candle data.
func (_self *PocketOption) Subscribe(asset string, durationSecs uint64) (*SubscriptionStream, error) {
	_pointer := _self.ffiObject.incrementPointer("*PocketOption")
	defer _self.ffiObject.decrementPointer()
	 res, err :=uniffiRustCallAsync[UniError](
        FfiConverterUniErrorINSTANCE,
		// completeFn
		func(handle C.uint64_t, status *C.RustCallStatus) unsafe.Pointer {
			res := C.ffi_binary_options_tools_uni_rust_future_complete_pointer(handle, status)
			return res
		},
		// liftFn
		func(ffi unsafe.Pointer) *SubscriptionStream {
			return FfiConverterSubscriptionStreamINSTANCE.Lift(ffi)
		},
		C.uniffi_binary_options_tools_uni_fn_method_pocketoption_subscribe(
		_pointer,FfiConverterStringINSTANCE.Lower(asset), FfiConverterUint64INSTANCE.Lower(durationSecs)),
		// pollFn
		func (handle C.uint64_t, continuation C.UniffiRustFutureContinuationCallback, data C.uint64_t) {
			C.ffi_binary_options_tools_uni_rust_future_poll_pointer(handle, continuation, data)
		},
		// freeFn
		func (handle C.uint64_t) {
			C.ffi_binary_options_tools_uni_rust_future_free_pointer(handle)
		},
	)

	return res, err 
}

// Places a trade.
//
// This is the core method for executing trades.
//
// # Arguments
//
// * `asset` - The symbol of the asset to trade (e.g., "EURUSD_otc").
// * `action` - The direction of the trade (`Action.Call` or `Action.Put`).
// * `time` - The duration of the trade in seconds.
// * `amount` - The amount to trade.
//
// # Returns
//
// A `Deal` object representing the completed trade.
func (_self *PocketOption) Trade(asset string, action Action, time uint32, amount float64) (Deal, error) {
	_pointer := _self.ffiObject.incrementPointer("*PocketOption")
	defer _self.ffiObject.decrementPointer()
	 res, err :=uniffiRustCallAsync[UniError](
        FfiConverterUniErrorINSTANCE,
		// completeFn
		func(handle C.uint64_t, status *C.RustCallStatus) RustBufferI {
			res := C.ffi_binary_options_tools_uni_rust_future_complete_rust_buffer(handle, status)
			return GoRustBuffer {
		inner: res,
	}
		},
		// liftFn
		func(ffi RustBufferI) Deal {
			return FfiConverterDealINSTANCE.Lift(ffi)
		},
		C.uniffi_binary_options_tools_uni_fn_method_pocketoption_trade(
		_pointer,FfiConverterStringINSTANCE.Lower(asset), FfiConverterActionINSTANCE.Lower(action), FfiConverterUint32INSTANCE.Lower(time), FfiConverterFloat64INSTANCE.Lower(amount)),
		// pollFn
		func (handle C.uint64_t, continuation C.UniffiRustFutureContinuationCallback, data C.uint64_t) {
			C.ffi_binary_options_tools_uni_rust_future_poll_rust_buffer(handle, continuation, data)
		},
		// freeFn
		func (handle C.uint64_t) {
			C.ffi_binary_options_tools_uni_rust_future_free_rust_buffer(handle)
		},
	)

	return res, err 
}

// Unsubscribes from real-time candle data for a specific asset.
func (_self *PocketOption) Unsubscribe(asset string) error {
	_pointer := _self.ffiObject.incrementPointer("*PocketOption")
	defer _self.ffiObject.decrementPointer()
	 _, err :=uniffiRustCallAsync[UniError](
        FfiConverterUniErrorINSTANCE,
		// completeFn
		func(handle C.uint64_t, status *C.RustCallStatus) struct{} {
			C.ffi_binary_options_tools_uni_rust_future_complete_void(handle, status)
			return struct{}{}
		},
		// liftFn
		func(_ struct{}) struct{} { return struct{}{} },
		C.uniffi_binary_options_tools_uni_fn_method_pocketoption_unsubscribe(
		_pointer,FfiConverterStringINSTANCE.Lower(asset)),
		// pollFn
		func (handle C.uint64_t, continuation C.UniffiRustFutureContinuationCallback, data C.uint64_t) {
			C.ffi_binary_options_tools_uni_rust_future_poll_void(handle, continuation, data)
		},
		// freeFn
		func (handle C.uint64_t) {
			C.ffi_binary_options_tools_uni_rust_future_free_void(handle)
		},
	)

	return err 
}
func (object *PocketOption) Destroy() {
	runtime.SetFinalizer(object, nil)
	object.ffiObject.destroy()
}

type FfiConverterPocketOption struct {}

var FfiConverterPocketOptionINSTANCE = FfiConverterPocketOption{}


func (c FfiConverterPocketOption) Lift(pointer unsafe.Pointer) *PocketOption {
	result := &PocketOption {
		newFfiObject(
			pointer,
			func(pointer unsafe.Pointer, status *C.RustCallStatus) unsafe.Pointer {
				return C.uniffi_binary_options_tools_uni_fn_clone_pocketoption(pointer, status)
			},
			func(pointer unsafe.Pointer, status *C.RustCallStatus) {
				C.uniffi_binary_options_tools_uni_fn_free_pocketoption(pointer, status)
			},
		),
	}
	runtime.SetFinalizer(result, (*PocketOption).Destroy)
	return result
}

func (c FfiConverterPocketOption) Read(reader io.Reader) *PocketOption {
	return c.Lift(unsafe.Pointer(uintptr(readUint64(reader))))
}

func (c FfiConverterPocketOption) Lower(value *PocketOption) unsafe.Pointer {
	// TODO: this is bad - all synchronization from ObjectRuntime.go is discarded here,
	// because the pointer will be decremented immediately after this function returns,
	// and someone will be left holding onto a non-locked pointer.
	pointer := value.ffiObject.incrementPointer("*PocketOption")
	defer value.ffiObject.decrementPointer()
	return pointer
	
}

func (c FfiConverterPocketOption) Write(writer io.Writer, value *PocketOption) {
	writeUint64(writer, uint64(uintptr(c.Lower(value))))
}

type FfiDestroyerPocketOption struct {}

func (_ FfiDestroyerPocketOption) Destroy(value *PocketOption) {
		value.Destroy()
}





// Represents a stream of subscription data.
//
// This object is returned by the `subscribe` method on the `PocketOption` client.
// It allows you to receive real-time data, such as candles, for a specific asset.
//
// # Rationale
//
// Since UniFFI does not support streams directly, this wrapper provides a way to
// consume the stream by repeatedly calling the `next` method.
type SubscriptionStreamInterface interface {
	// Retrieves the next item from the stream.
	//
	// This method should be called in a loop to consume the data from the stream.
	// It will return `None` when the stream is closed.
	//
	// # Returns
	//
	// An optional `Candle` object. It will be `None` if the stream has finished.
	//
	// # Examples
	//
	// ## Python
	// ```python
	// import asyncio
	//
	// async def main():
	// # ... (get api object)
	// stream = await api.subscribe("EURUSD_otc", 5)
	// while True:
	// candle = await stream.next()
	// if candle is None:
	// break
	// print(f"New candle: {candle}")
	//
	// asyncio.run(main())
	// ```
	//
	// ## Swift
	// ```swift
	// func subscribe() async {
	// // ... (get api object)
	// let stream = try! await api.subscribe(asset: "EURUSD_otc", durationSecs: 5)
	// while let candle = try! await stream.next() {
	// print("New candle: \(candle)")
	// }
	// }
	// ```
	Next() (Candle, error)
}
// Represents a stream of subscription data.
//
// This object is returned by the `subscribe` method on the `PocketOption` client.
// It allows you to receive real-time data, such as candles, for a specific asset.
//
// # Rationale
//
// Since UniFFI does not support streams directly, this wrapper provides a way to
// consume the stream by repeatedly calling the `next` method.
type SubscriptionStream struct {
	ffiObject FfiObject
}




// Retrieves the next item from the stream.
//
// This method should be called in a loop to consume the data from the stream.
// It will return `None` when the stream is closed.
//
// # Returns
//
// An optional `Candle` object. It will be `None` if the stream has finished.
//
// # Examples
//
// ## Python
// ```python
// import asyncio
//
// async def main():
// # ... (get api object)
// stream = await api.subscribe("EURUSD_otc", 5)
// while True:
// candle = await stream.next()
// if candle is None:
// break
// print(f"New candle: {candle}")
//
// asyncio.run(main())
// ```
//
// ## Swift
// ```swift
// func subscribe() async {
// // ... (get api object)
// let stream = try! await api.subscribe(asset: "EURUSD_otc", durationSecs: 5)
// while let candle = try! await stream.next() {
// print("New candle: \(candle)")
// }
// }
// ```
func (_self *SubscriptionStream) Next() (Candle, error) {
	_pointer := _self.ffiObject.incrementPointer("*SubscriptionStream")
	defer _self.ffiObject.decrementPointer()
	 res, err :=uniffiRustCallAsync[UniError](
        FfiConverterUniErrorINSTANCE,
		// completeFn
		func(handle C.uint64_t, status *C.RustCallStatus) RustBufferI {
			res := C.ffi_binary_options_tools_uni_rust_future_complete_rust_buffer(handle, status)
			return GoRustBuffer {
		inner: res,
	}
		},
		// liftFn
		func(ffi RustBufferI) Candle {
			return FfiConverterCandleINSTANCE.Lift(ffi)
		},
		C.uniffi_binary_options_tools_uni_fn_method_subscriptionstream_next(
		_pointer,),
		// pollFn
		func (handle C.uint64_t, continuation C.UniffiRustFutureContinuationCallback, data C.uint64_t) {
			C.ffi_binary_options_tools_uni_rust_future_poll_rust_buffer(handle, continuation, data)
		},
		// freeFn
		func (handle C.uint64_t) {
			C.ffi_binary_options_tools_uni_rust_future_free_rust_buffer(handle)
		},
	)

	return res, err 
}
func (object *SubscriptionStream) Destroy() {
	runtime.SetFinalizer(object, nil)
	object.ffiObject.destroy()
}

type FfiConverterSubscriptionStream struct {}

var FfiConverterSubscriptionStreamINSTANCE = FfiConverterSubscriptionStream{}


func (c FfiConverterSubscriptionStream) Lift(pointer unsafe.Pointer) *SubscriptionStream {
	result := &SubscriptionStream {
		newFfiObject(
			pointer,
			func(pointer unsafe.Pointer, status *C.RustCallStatus) unsafe.Pointer {
				return C.uniffi_binary_options_tools_uni_fn_clone_subscriptionstream(pointer, status)
			},
			func(pointer unsafe.Pointer, status *C.RustCallStatus) {
				C.uniffi_binary_options_tools_uni_fn_free_subscriptionstream(pointer, status)
			},
		),
	}
	runtime.SetFinalizer(result, (*SubscriptionStream).Destroy)
	return result
}

func (c FfiConverterSubscriptionStream) Read(reader io.Reader) *SubscriptionStream {
	return c.Lift(unsafe.Pointer(uintptr(readUint64(reader))))
}

func (c FfiConverterSubscriptionStream) Lower(value *SubscriptionStream) unsafe.Pointer {
	// TODO: this is bad - all synchronization from ObjectRuntime.go is discarded here,
	// because the pointer will be decremented immediately after this function returns,
	// and someone will be left holding onto a non-locked pointer.
	pointer := value.ffiObject.incrementPointer("*SubscriptionStream")
	defer value.ffiObject.decrementPointer()
	return pointer
	
}

func (c FfiConverterSubscriptionStream) Write(writer io.Writer, value *SubscriptionStream) {
	writeUint64(writer, uint64(uintptr(c.Lower(value))))
}

type FfiDestroyerSubscriptionStream struct {}

func (_ FfiDestroyerSubscriptionStream) Destroy(value *SubscriptionStream) {
		value.Destroy()
}





// Represents a financial asset that can be traded.
//
// This struct contains all the information about a specific asset, such as its name, symbol,
// payout, and whether it's currently active.
//
// # Examples
//
// ## Python
// ```python
// from binaryoptionstoolsuni import Asset
//
// # This is an example of how you might receive an Asset object
// # from the API. You would not typically construct this yourself.
// eurusd = Asset(id=1, name="EUR/USD", symbol="EURUSD_otc", is_otc=True, is_active=True, payout=85, allowed_candles=[], asset_type=AssetType.CURRENCY)
// print(eurusd.name)
// ```
type Asset struct {
	Id int32
	Name string
	Symbol string
	IsOtc bool
	IsActive bool
	Payout int32
	AllowedCandles []CandleLength
	AssetType AssetType
}

func (r *Asset) Destroy() {
		FfiDestroyerInt32{}.Destroy(r.Id);
		FfiDestroyerString{}.Destroy(r.Name);
		FfiDestroyerString{}.Destroy(r.Symbol);
		FfiDestroyerBool{}.Destroy(r.IsOtc);
		FfiDestroyerBool{}.Destroy(r.IsActive);
		FfiDestroyerInt32{}.Destroy(r.Payout);
		FfiDestroyerSequenceCandleLength{}.Destroy(r.AllowedCandles);
		FfiDestroyerAssetType{}.Destroy(r.AssetType);
}

type FfiConverterAsset struct {}

var FfiConverterAssetINSTANCE = FfiConverterAsset{}

func (c FfiConverterAsset) Lift(rb RustBufferI) Asset {
	return LiftFromRustBuffer[Asset](c, rb)
}

func (c FfiConverterAsset) Read(reader io.Reader) Asset {
	return Asset {
			FfiConverterInt32INSTANCE.Read(reader),
			FfiConverterStringINSTANCE.Read(reader),
			FfiConverterStringINSTANCE.Read(reader),
			FfiConverterBoolINSTANCE.Read(reader),
			FfiConverterBoolINSTANCE.Read(reader),
			FfiConverterInt32INSTANCE.Read(reader),
			FfiConverterSequenceCandleLengthINSTANCE.Read(reader),
			FfiConverterAssetTypeINSTANCE.Read(reader),
	}
}

func (c FfiConverterAsset) Lower(value Asset) C.RustBuffer {
	return LowerIntoRustBuffer[Asset](c, value)
}

func (c FfiConverterAsset) Write(writer io.Writer, value Asset) {
		FfiConverterInt32INSTANCE.Write(writer, value.Id);
		FfiConverterStringINSTANCE.Write(writer, value.Name);
		FfiConverterStringINSTANCE.Write(writer, value.Symbol);
		FfiConverterBoolINSTANCE.Write(writer, value.IsOtc);
		FfiConverterBoolINSTANCE.Write(writer, value.IsActive);
		FfiConverterInt32INSTANCE.Write(writer, value.Payout);
		FfiConverterSequenceCandleLengthINSTANCE.Write(writer, value.AllowedCandles);
		FfiConverterAssetTypeINSTANCE.Write(writer, value.AssetType);
}

type FfiDestroyerAsset struct {}

func (_ FfiDestroyerAsset) Destroy(value Asset) {
	value.Destroy()
}


// Represents a single candle in a price chart.
//
// A candle represents the price movement of an asset over a specific time period.
// It contains the open, high, low, and close (OHLC) prices for that period.
//
// # Examples
//
// ## Python
// ```python
// from binaryoptionstoolsuni import Candle
//
// # This is an example of how you might receive a Candle object
// # from the API.
// candle = ... # receive from api.get_candles() or stream.next()
// print(f"Candle for {candle.symbol} at {candle.timestamp}: O={candle.open}, H={candle.high}, L={candle.low}, C={candle.close}")
// ```
type Candle struct {
	Symbol string
	Timestamp int64
	Open float64
	High float64
	Low float64
	Close float64
	Volume *float64
}

func (r *Candle) Destroy() {
		FfiDestroyerString{}.Destroy(r.Symbol);
		FfiDestroyerInt64{}.Destroy(r.Timestamp);
		FfiDestroyerFloat64{}.Destroy(r.Open);
		FfiDestroyerFloat64{}.Destroy(r.High);
		FfiDestroyerFloat64{}.Destroy(r.Low);
		FfiDestroyerFloat64{}.Destroy(r.Close);
		FfiDestroyerOptionalFloat64{}.Destroy(r.Volume);
}

type FfiConverterCandle struct {}

var FfiConverterCandleINSTANCE = FfiConverterCandle{}

func (c FfiConverterCandle) Lift(rb RustBufferI) Candle {
	return LiftFromRustBuffer[Candle](c, rb)
}

func (c FfiConverterCandle) Read(reader io.Reader) Candle {
	return Candle {
			FfiConverterStringINSTANCE.Read(reader),
			FfiConverterInt64INSTANCE.Read(reader),
			FfiConverterFloat64INSTANCE.Read(reader),
			FfiConverterFloat64INSTANCE.Read(reader),
			FfiConverterFloat64INSTANCE.Read(reader),
			FfiConverterFloat64INSTANCE.Read(reader),
			FfiConverterOptionalFloat64INSTANCE.Read(reader),
	}
}

func (c FfiConverterCandle) Lower(value Candle) C.RustBuffer {
	return LowerIntoRustBuffer[Candle](c, value)
}

func (c FfiConverterCandle) Write(writer io.Writer, value Candle) {
		FfiConverterStringINSTANCE.Write(writer, value.Symbol);
		FfiConverterInt64INSTANCE.Write(writer, value.Timestamp);
		FfiConverterFloat64INSTANCE.Write(writer, value.Open);
		FfiConverterFloat64INSTANCE.Write(writer, value.High);
		FfiConverterFloat64INSTANCE.Write(writer, value.Low);
		FfiConverterFloat64INSTANCE.Write(writer, value.Close);
		FfiConverterOptionalFloat64INSTANCE.Write(writer, value.Volume);
}

type FfiDestroyerCandle struct {}

func (_ FfiDestroyerCandle) Destroy(value Candle) {
	value.Destroy()
}


// Represents the duration of a candle.
//
// This struct is a simple wrapper around a `u32` that represents the candle duration in seconds.
// It is used in the `Asset` struct to specify the allowed candle lengths for an asset.
//
// # Examples
//
// ## Python
// ```python
// from binaryoptionstoolsuni import CandleLength
//
// five_second_candle = CandleLength(time=5)
// ```
type CandleLength struct {
	Time uint32
}

func (r *CandleLength) Destroy() {
		FfiDestroyerUint32{}.Destroy(r.Time);
}

type FfiConverterCandleLength struct {}

var FfiConverterCandleLengthINSTANCE = FfiConverterCandleLength{}

func (c FfiConverterCandleLength) Lift(rb RustBufferI) CandleLength {
	return LiftFromRustBuffer[CandleLength](c, rb)
}

func (c FfiConverterCandleLength) Read(reader io.Reader) CandleLength {
	return CandleLength {
			FfiConverterUint32INSTANCE.Read(reader),
	}
}

func (c FfiConverterCandleLength) Lower(value CandleLength) C.RustBuffer {
	return LowerIntoRustBuffer[CandleLength](c, value)
}

func (c FfiConverterCandleLength) Write(writer io.Writer, value CandleLength) {
		FfiConverterUint32INSTANCE.Write(writer, value.Time);
}

type FfiDestroyerCandleLength struct {}

func (_ FfiDestroyerCandleLength) Destroy(value CandleLength) {
	value.Destroy()
}


// Represents a completed trade.
//
// This struct contains all the information about a trade that has been opened and subsequently closed.
// It includes details such as the open and close prices, profit, and timestamps.
//
// # Examples
//
// ## Python
// ```python
// from binaryoptionstoolsuni import Deal
//
// # This is an example of how you might receive a Deal object
// # from the API after a trade is completed.
// # You would not typically construct this yourself.
// deal = ... # receive from api.result()
// print(f"Trade {deal.id} on {deal.asset} resulted in a profit of {deal.profit}")
// ```
type Deal struct {
	Id string
	OpenTime string
	CloseTime string
	OpenTimestamp int64
	CloseTimestamp int64
	Uid uint64
	RequestId *string
	Amount float64
	Profit float64
	PercentProfit int32
	PercentLoss int32
	OpenPrice float64
	ClosePrice float64
	Command int32
	Asset string
	IsDemo uint32
	CopyTicket string
	OpenMs int32
	CloseMs *int32
	OptionType int32
	IsRollover *bool
	IsCopySignal *bool
	IsAi *bool
	Currency string
	AmountUsd *float64
	AmountUsd2 *float64
}

func (r *Deal) Destroy() {
		FfiDestroyerString{}.Destroy(r.Id);
		FfiDestroyerString{}.Destroy(r.OpenTime);
		FfiDestroyerString{}.Destroy(r.CloseTime);
		FfiDestroyerInt64{}.Destroy(r.OpenTimestamp);
		FfiDestroyerInt64{}.Destroy(r.CloseTimestamp);
		FfiDestroyerUint64{}.Destroy(r.Uid);
		FfiDestroyerOptionalString{}.Destroy(r.RequestId);
		FfiDestroyerFloat64{}.Destroy(r.Amount);
		FfiDestroyerFloat64{}.Destroy(r.Profit);
		FfiDestroyerInt32{}.Destroy(r.PercentProfit);
		FfiDestroyerInt32{}.Destroy(r.PercentLoss);
		FfiDestroyerFloat64{}.Destroy(r.OpenPrice);
		FfiDestroyerFloat64{}.Destroy(r.ClosePrice);
		FfiDestroyerInt32{}.Destroy(r.Command);
		FfiDestroyerString{}.Destroy(r.Asset);
		FfiDestroyerUint32{}.Destroy(r.IsDemo);
		FfiDestroyerString{}.Destroy(r.CopyTicket);
		FfiDestroyerInt32{}.Destroy(r.OpenMs);
		FfiDestroyerOptionalInt32{}.Destroy(r.CloseMs);
		FfiDestroyerInt32{}.Destroy(r.OptionType);
		FfiDestroyerOptionalBool{}.Destroy(r.IsRollover);
		FfiDestroyerOptionalBool{}.Destroy(r.IsCopySignal);
		FfiDestroyerOptionalBool{}.Destroy(r.IsAi);
		FfiDestroyerString{}.Destroy(r.Currency);
		FfiDestroyerOptionalFloat64{}.Destroy(r.AmountUsd);
		FfiDestroyerOptionalFloat64{}.Destroy(r.AmountUsd2);
}

type FfiConverterDeal struct {}

var FfiConverterDealINSTANCE = FfiConverterDeal{}

func (c FfiConverterDeal) Lift(rb RustBufferI) Deal {
	return LiftFromRustBuffer[Deal](c, rb)
}

func (c FfiConverterDeal) Read(reader io.Reader) Deal {
	return Deal {
			FfiConverterStringINSTANCE.Read(reader),
			FfiConverterStringINSTANCE.Read(reader),
			FfiConverterStringINSTANCE.Read(reader),
			FfiConverterInt64INSTANCE.Read(reader),
			FfiConverterInt64INSTANCE.Read(reader),
			FfiConverterUint64INSTANCE.Read(reader),
			FfiConverterOptionalStringINSTANCE.Read(reader),
			FfiConverterFloat64INSTANCE.Read(reader),
			FfiConverterFloat64INSTANCE.Read(reader),
			FfiConverterInt32INSTANCE.Read(reader),
			FfiConverterInt32INSTANCE.Read(reader),
			FfiConverterFloat64INSTANCE.Read(reader),
			FfiConverterFloat64INSTANCE.Read(reader),
			FfiConverterInt32INSTANCE.Read(reader),
			FfiConverterStringINSTANCE.Read(reader),
			FfiConverterUint32INSTANCE.Read(reader),
			FfiConverterStringINSTANCE.Read(reader),
			FfiConverterInt32INSTANCE.Read(reader),
			FfiConverterOptionalInt32INSTANCE.Read(reader),
			FfiConverterInt32INSTANCE.Read(reader),
			FfiConverterOptionalBoolINSTANCE.Read(reader),
			FfiConverterOptionalBoolINSTANCE.Read(reader),
			FfiConverterOptionalBoolINSTANCE.Read(reader),
			FfiConverterStringINSTANCE.Read(reader),
			FfiConverterOptionalFloat64INSTANCE.Read(reader),
			FfiConverterOptionalFloat64INSTANCE.Read(reader),
	}
}

func (c FfiConverterDeal) Lower(value Deal) C.RustBuffer {
	return LowerIntoRustBuffer[Deal](c, value)
}

func (c FfiConverterDeal) Write(writer io.Writer, value Deal) {
		FfiConverterStringINSTANCE.Write(writer, value.Id);
		FfiConverterStringINSTANCE.Write(writer, value.OpenTime);
		FfiConverterStringINSTANCE.Write(writer, value.CloseTime);
		FfiConverterInt64INSTANCE.Write(writer, value.OpenTimestamp);
		FfiConverterInt64INSTANCE.Write(writer, value.CloseTimestamp);
		FfiConverterUint64INSTANCE.Write(writer, value.Uid);
		FfiConverterOptionalStringINSTANCE.Write(writer, value.RequestId);
		FfiConverterFloat64INSTANCE.Write(writer, value.Amount);
		FfiConverterFloat64INSTANCE.Write(writer, value.Profit);
		FfiConverterInt32INSTANCE.Write(writer, value.PercentProfit);
		FfiConverterInt32INSTANCE.Write(writer, value.PercentLoss);
		FfiConverterFloat64INSTANCE.Write(writer, value.OpenPrice);
		FfiConverterFloat64INSTANCE.Write(writer, value.ClosePrice);
		FfiConverterInt32INSTANCE.Write(writer, value.Command);
		FfiConverterStringINSTANCE.Write(writer, value.Asset);
		FfiConverterUint32INSTANCE.Write(writer, value.IsDemo);
		FfiConverterStringINSTANCE.Write(writer, value.CopyTicket);
		FfiConverterInt32INSTANCE.Write(writer, value.OpenMs);
		FfiConverterOptionalInt32INSTANCE.Write(writer, value.CloseMs);
		FfiConverterInt32INSTANCE.Write(writer, value.OptionType);
		FfiConverterOptionalBoolINSTANCE.Write(writer, value.IsRollover);
		FfiConverterOptionalBoolINSTANCE.Write(writer, value.IsCopySignal);
		FfiConverterOptionalBoolINSTANCE.Write(writer, value.IsAi);
		FfiConverterStringINSTANCE.Write(writer, value.Currency);
		FfiConverterOptionalFloat64INSTANCE.Write(writer, value.AmountUsd);
		FfiConverterOptionalFloat64INSTANCE.Write(writer, value.AmountUsd2);
}

type FfiDestroyerDeal struct {}

func (_ FfiDestroyerDeal) Destroy(value Deal) {
	value.Destroy()
}



// Represents the action to take in a trade.
//
// This enum is used to specify whether a trade is a "Call" (buy) or a "Put" (sell).
// It's a fundamental concept in binary options trading.
//
// # Examples
//
// ## Python
// ```python
// from binaryoptionstoolsuni import Action
//
// buy_action = Action.CALL
// sell_action = Action.PUT
// ```
//
// ## Swift
// ```swift
// import binaryoptionstoolsuni
//
// let buyAction = Action.call
// let sellAction = Action.put
// ```
//
// ## Kotlin
// ```kotlin
// import uniffi.binaryoptionstoolsuni.Action
//
// val buyAction = Action.CALL
// val sellAction = Action.PUT
// ```
//
// ## C#
// ```csharp
// using UniFFI.BinaryOptionsToolsUni;
//
// var buyAction = Action.Call;
// var sellAction = Action.Put;
// ```
//
// ## Go
// ```go
// import "github.com/your-repo/binaryoptionstoolsuni"
//
// var buyAction = binaryoptionstoolsuni.ActionCall
// var sellAction = binaryoptionstoolsuni.ActionPut
// ```
type Action uint

const (
	ActionCall Action = 1
	ActionPut Action = 2
)

type FfiConverterAction struct {}

var FfiConverterActionINSTANCE = FfiConverterAction{}

func (c FfiConverterAction) Lift(rb RustBufferI) Action {
	return LiftFromRustBuffer[Action](c, rb)
}

func (c FfiConverterAction) Lower(value Action) C.RustBuffer {
	return LowerIntoRustBuffer[Action](c, value)
}
func (FfiConverterAction) Read(reader io.Reader) Action {
	id := readInt32(reader)
	return Action(id)
}

func (FfiConverterAction) Write(writer io.Writer, value Action) {
	writeInt32(writer, int32(value))
}

type FfiDestroyerAction struct {}

func (_ FfiDestroyerAction) Destroy(value Action) {
}




// Represents the type of an asset.
//
// This enum is used to categorize assets into different types, such as stocks, currencies, etc.
// This information can be useful for filtering and organizing assets.
//
// # Examples
//
// ## Python
// ```python
// from binaryoptionstoolsuni import AssetType
//
// asset_type = AssetType.CURRENCY
// ```
type AssetType uint

const (
	AssetTypeStock AssetType = 1
	AssetTypeCurrency AssetType = 2
	AssetTypeCommodity AssetType = 3
	AssetTypeCryptocurrency AssetType = 4
	AssetTypeIndex AssetType = 5
)

type FfiConverterAssetType struct {}

var FfiConverterAssetTypeINSTANCE = FfiConverterAssetType{}

func (c FfiConverterAssetType) Lift(rb RustBufferI) AssetType {
	return LiftFromRustBuffer[AssetType](c, rb)
}

func (c FfiConverterAssetType) Lower(value AssetType) C.RustBuffer {
	return LowerIntoRustBuffer[AssetType](c, value)
}
func (FfiConverterAssetType) Read(reader io.Reader) AssetType {
	id := readInt32(reader)
	return AssetType(id)
}

func (FfiConverterAssetType) Write(writer io.Writer, value AssetType) {
	writeInt32(writer, int32(value))
}

type FfiDestroyerAssetType struct {}

func (_ FfiDestroyerAssetType) Destroy(value AssetType) {
}


type UniError struct {
	err error
}

// Convience method to turn *UniError into error
// Avoiding treating nil pointer as non nil error interface
func (err *UniError) AsError() error {
	if err == nil {
		return nil
	} else {
		return err
	}
}

func (err UniError) Error() string {
	return fmt.Sprintf("UniError: %s", err.err.Error())
}

func (err UniError) Unwrap() error {
	return err.err
}

// Err* are used for checking error type with `errors.Is`
var ErrUniErrorBinaryOptions = fmt.Errorf("UniErrorBinaryOptions")

// Variant structs
type UniErrorBinaryOptions struct {
	Field0 string
}
func NewUniErrorBinaryOptions(
	var0 string,
) *UniError {
	return &UniError { err: &UniErrorBinaryOptions {
			Field0: var0,} }
}

func (e UniErrorBinaryOptions) destroy() {
		FfiDestroyerString{}.Destroy(e.Field0)
}


func (err UniErrorBinaryOptions) Error() string {
	return fmt.Sprint("BinaryOptions",
		": ",
		
		"Field0=",
		err.Field0,
	)
}

func (self UniErrorBinaryOptions) Is(target error) bool {
	return target == ErrUniErrorBinaryOptions
}

type FfiConverterUniError struct{}

var FfiConverterUniErrorINSTANCE = FfiConverterUniError{}

func (c FfiConverterUniError) Lift(eb RustBufferI) *UniError {
	return LiftFromRustBuffer[*UniError](c, eb)
}

func (c FfiConverterUniError) Lower(value *UniError) C.RustBuffer {
	return LowerIntoRustBuffer[*UniError](c, value)
}

func (c FfiConverterUniError) Read(reader io.Reader) *UniError {
	errorID := readUint32(reader)

	switch errorID {
	case 1:
		return &UniError{ &UniErrorBinaryOptions{
			Field0: FfiConverterStringINSTANCE.Read(reader),
		}}
	default:
		panic(fmt.Sprintf("Unknown error code %d in FfiConverterUniError.Read()", errorID))
	}
}

func (c FfiConverterUniError) Write(writer io.Writer, value *UniError) {
	switch variantValue := value.err.(type) {
		case *UniErrorBinaryOptions:
			writeInt32(writer, 1)
			FfiConverterStringINSTANCE.Write(writer, variantValue.Field0)
		default:
			_ = variantValue
			panic(fmt.Sprintf("invalid error value `%v` in FfiConverterUniError.Write", value))
	}
}

type FfiDestroyerUniError struct {}

func (_ FfiDestroyerUniError) Destroy(value *UniError) {
	switch variantValue := value.err.(type) {
		case UniErrorBinaryOptions:
			variantValue.destroy()
		default:
			_ = variantValue
			panic(fmt.Sprintf("invalid error value `%v` in FfiDestroyerUniError.Destroy", value))
	}
}




type FfiConverterOptionalInt32 struct{}

var FfiConverterOptionalInt32INSTANCE = FfiConverterOptionalInt32{}

func (c FfiConverterOptionalInt32) Lift(rb RustBufferI) *int32 {
	return LiftFromRustBuffer[*int32](c, rb)
}

func (_ FfiConverterOptionalInt32) Read(reader io.Reader) *int32 {
	if readInt8(reader) == 0 {
		return nil
	}
	temp := FfiConverterInt32INSTANCE.Read(reader)
	return &temp
}

func (c FfiConverterOptionalInt32) Lower(value *int32) C.RustBuffer {
	return LowerIntoRustBuffer[*int32](c, value)
}

func (_ FfiConverterOptionalInt32) Write(writer io.Writer, value *int32) {
	if value == nil {
		writeInt8(writer, 0)
	} else {
		writeInt8(writer, 1)
		FfiConverterInt32INSTANCE.Write(writer, *value)
	}
}

type FfiDestroyerOptionalInt32 struct {}

func (_ FfiDestroyerOptionalInt32) Destroy(value *int32) {
	if value != nil {
		FfiDestroyerInt32{}.Destroy(*value)
	}
}



type FfiConverterOptionalFloat64 struct{}

var FfiConverterOptionalFloat64INSTANCE = FfiConverterOptionalFloat64{}

func (c FfiConverterOptionalFloat64) Lift(rb RustBufferI) *float64 {
	return LiftFromRustBuffer[*float64](c, rb)
}

func (_ FfiConverterOptionalFloat64) Read(reader io.Reader) *float64 {
	if readInt8(reader) == 0 {
		return nil
	}
	temp := FfiConverterFloat64INSTANCE.Read(reader)
	return &temp
}

func (c FfiConverterOptionalFloat64) Lower(value *float64) C.RustBuffer {
	return LowerIntoRustBuffer[*float64](c, value)
}

func (_ FfiConverterOptionalFloat64) Write(writer io.Writer, value *float64) {
	if value == nil {
		writeInt8(writer, 0)
	} else {
		writeInt8(writer, 1)
		FfiConverterFloat64INSTANCE.Write(writer, *value)
	}
}

type FfiDestroyerOptionalFloat64 struct {}

func (_ FfiDestroyerOptionalFloat64) Destroy(value *float64) {
	if value != nil {
		FfiDestroyerFloat64{}.Destroy(*value)
	}
}



type FfiConverterOptionalBool struct{}

var FfiConverterOptionalBoolINSTANCE = FfiConverterOptionalBool{}

func (c FfiConverterOptionalBool) Lift(rb RustBufferI) *bool {
	return LiftFromRustBuffer[*bool](c, rb)
}

func (_ FfiConverterOptionalBool) Read(reader io.Reader) *bool {
	if readInt8(reader) == 0 {
		return nil
	}
	temp := FfiConverterBoolINSTANCE.Read(reader)
	return &temp
}

func (c FfiConverterOptionalBool) Lower(value *bool) C.RustBuffer {
	return LowerIntoRustBuffer[*bool](c, value)
}

func (_ FfiConverterOptionalBool) Write(writer io.Writer, value *bool) {
	if value == nil {
		writeInt8(writer, 0)
	} else {
		writeInt8(writer, 1)
		FfiConverterBoolINSTANCE.Write(writer, *value)
	}
}

type FfiDestroyerOptionalBool struct {}

func (_ FfiDestroyerOptionalBool) Destroy(value *bool) {
	if value != nil {
		FfiDestroyerBool{}.Destroy(*value)
	}
}



type FfiConverterOptionalString struct{}

var FfiConverterOptionalStringINSTANCE = FfiConverterOptionalString{}

func (c FfiConverterOptionalString) Lift(rb RustBufferI) *string {
	return LiftFromRustBuffer[*string](c, rb)
}

func (_ FfiConverterOptionalString) Read(reader io.Reader) *string {
	if readInt8(reader) == 0 {
		return nil
	}
	temp := FfiConverterStringINSTANCE.Read(reader)
	return &temp
}

func (c FfiConverterOptionalString) Lower(value *string) C.RustBuffer {
	return LowerIntoRustBuffer[*string](c, value)
}

func (_ FfiConverterOptionalString) Write(writer io.Writer, value *string) {
	if value == nil {
		writeInt8(writer, 0)
	} else {
		writeInt8(writer, 1)
		FfiConverterStringINSTANCE.Write(writer, *value)
	}
}

type FfiDestroyerOptionalString struct {}

func (_ FfiDestroyerOptionalString) Destroy(value *string) {
	if value != nil {
		FfiDestroyerString{}.Destroy(*value)
	}
}



type FfiConverterOptionalSequenceAsset struct{}

var FfiConverterOptionalSequenceAssetINSTANCE = FfiConverterOptionalSequenceAsset{}

func (c FfiConverterOptionalSequenceAsset) Lift(rb RustBufferI) *[]Asset {
	return LiftFromRustBuffer[*[]Asset](c, rb)
}

func (_ FfiConverterOptionalSequenceAsset) Read(reader io.Reader) *[]Asset {
	if readInt8(reader) == 0 {
		return nil
	}
	temp := FfiConverterSequenceAssetINSTANCE.Read(reader)
	return &temp
}

func (c FfiConverterOptionalSequenceAsset) Lower(value *[]Asset) C.RustBuffer {
	return LowerIntoRustBuffer[*[]Asset](c, value)
}

func (_ FfiConverterOptionalSequenceAsset) Write(writer io.Writer, value *[]Asset) {
	if value == nil {
		writeInt8(writer, 0)
	} else {
		writeInt8(writer, 1)
		FfiConverterSequenceAssetINSTANCE.Write(writer, *value)
	}
}

type FfiDestroyerOptionalSequenceAsset struct {}

func (_ FfiDestroyerOptionalSequenceAsset) Destroy(value *[]Asset) {
	if value != nil {
		FfiDestroyerSequenceAsset{}.Destroy(*value)
	}
}



type FfiConverterSequenceAsset struct{}

var FfiConverterSequenceAssetINSTANCE = FfiConverterSequenceAsset{}

func (c FfiConverterSequenceAsset) Lift(rb RustBufferI) []Asset {
	return LiftFromRustBuffer[[]Asset](c, rb)
}

func (c FfiConverterSequenceAsset) Read(reader io.Reader) []Asset {
	length := readInt32(reader)
	if length == 0 {
		return nil
	}
	result := make([]Asset, 0, length)
	for i := int32(0); i < length; i++ {
		result = append(result, FfiConverterAssetINSTANCE.Read(reader))
	}
	return result
}

func (c FfiConverterSequenceAsset) Lower(value []Asset) C.RustBuffer {
	return LowerIntoRustBuffer[[]Asset](c, value)
}

func (c FfiConverterSequenceAsset) Write(writer io.Writer, value []Asset) {
	if len(value) > math.MaxInt32 {
		panic("[]Asset is too large to fit into Int32")
	}

	writeInt32(writer, int32(len(value)))
	for _, item := range value {
		FfiConverterAssetINSTANCE.Write(writer, item)
	}
}

type FfiDestroyerSequenceAsset struct {}

func (FfiDestroyerSequenceAsset) Destroy(sequence []Asset) {
	for _, value := range sequence {
		FfiDestroyerAsset{}.Destroy(value)	
	}
}



type FfiConverterSequenceCandle struct{}

var FfiConverterSequenceCandleINSTANCE = FfiConverterSequenceCandle{}

func (c FfiConverterSequenceCandle) Lift(rb RustBufferI) []Candle {
	return LiftFromRustBuffer[[]Candle](c, rb)
}

func (c FfiConverterSequenceCandle) Read(reader io.Reader) []Candle {
	length := readInt32(reader)
	if length == 0 {
		return nil
	}
	result := make([]Candle, 0, length)
	for i := int32(0); i < length; i++ {
		result = append(result, FfiConverterCandleINSTANCE.Read(reader))
	}
	return result
}

func (c FfiConverterSequenceCandle) Lower(value []Candle) C.RustBuffer {
	return LowerIntoRustBuffer[[]Candle](c, value)
}

func (c FfiConverterSequenceCandle) Write(writer io.Writer, value []Candle) {
	if len(value) > math.MaxInt32 {
		panic("[]Candle is too large to fit into Int32")
	}

	writeInt32(writer, int32(len(value)))
	for _, item := range value {
		FfiConverterCandleINSTANCE.Write(writer, item)
	}
}

type FfiDestroyerSequenceCandle struct {}

func (FfiDestroyerSequenceCandle) Destroy(sequence []Candle) {
	for _, value := range sequence {
		FfiDestroyerCandle{}.Destroy(value)	
	}
}



type FfiConverterSequenceCandleLength struct{}

var FfiConverterSequenceCandleLengthINSTANCE = FfiConverterSequenceCandleLength{}

func (c FfiConverterSequenceCandleLength) Lift(rb RustBufferI) []CandleLength {
	return LiftFromRustBuffer[[]CandleLength](c, rb)
}

func (c FfiConverterSequenceCandleLength) Read(reader io.Reader) []CandleLength {
	length := readInt32(reader)
	if length == 0 {
		return nil
	}
	result := make([]CandleLength, 0, length)
	for i := int32(0); i < length; i++ {
		result = append(result, FfiConverterCandleLengthINSTANCE.Read(reader))
	}
	return result
}

func (c FfiConverterSequenceCandleLength) Lower(value []CandleLength) C.RustBuffer {
	return LowerIntoRustBuffer[[]CandleLength](c, value)
}

func (c FfiConverterSequenceCandleLength) Write(writer io.Writer, value []CandleLength) {
	if len(value) > math.MaxInt32 {
		panic("[]CandleLength is too large to fit into Int32")
	}

	writeInt32(writer, int32(len(value)))
	for _, item := range value {
		FfiConverterCandleLengthINSTANCE.Write(writer, item)
	}
}

type FfiDestroyerSequenceCandleLength struct {}

func (FfiDestroyerSequenceCandleLength) Destroy(sequence []CandleLength) {
	for _, value := range sequence {
		FfiDestroyerCandleLength{}.Destroy(value)	
	}
}



type FfiConverterSequenceDeal struct{}

var FfiConverterSequenceDealINSTANCE = FfiConverterSequenceDeal{}

func (c FfiConverterSequenceDeal) Lift(rb RustBufferI) []Deal {
	return LiftFromRustBuffer[[]Deal](c, rb)
}

func (c FfiConverterSequenceDeal) Read(reader io.Reader) []Deal {
	length := readInt32(reader)
	if length == 0 {
		return nil
	}
	result := make([]Deal, 0, length)
	for i := int32(0); i < length; i++ {
		result = append(result, FfiConverterDealINSTANCE.Read(reader))
	}
	return result
}

func (c FfiConverterSequenceDeal) Lower(value []Deal) C.RustBuffer {
	return LowerIntoRustBuffer[[]Deal](c, value)
}

func (c FfiConverterSequenceDeal) Write(writer io.Writer, value []Deal) {
	if len(value) > math.MaxInt32 {
		panic("[]Deal is too large to fit into Int32")
	}

	writeInt32(writer, int32(len(value)))
	for _, item := range value {
		FfiConverterDealINSTANCE.Write(writer, item)
	}
}

type FfiDestroyerSequenceDeal struct {}

func (FfiDestroyerSequenceDeal) Destroy(sequence []Deal) {
	for _, value := range sequence {
		FfiDestroyerDeal{}.Destroy(value)	
	}
}


const (
	uniffiRustFuturePollReady      int8 = 0
	uniffiRustFuturePollMaybeReady int8 = 1
)

type rustFuturePollFunc func(C.uint64_t, C.UniffiRustFutureContinuationCallback, C.uint64_t)
type rustFutureCompleteFunc[T any] func(C.uint64_t, *C.RustCallStatus) T
type rustFutureFreeFunc func(C.uint64_t)

//export binary_options_tools_uni_uniffiFutureContinuationCallback
func binary_options_tools_uni_uniffiFutureContinuationCallback(data C.uint64_t, pollResult C.int8_t) {
	h := cgo.Handle(uintptr(data))
	waiter := h.Value().(chan int8)
	waiter <- int8(pollResult)
}

func uniffiRustCallAsync[E any, T any, F any](
	errConverter BufReader[*E],
	completeFunc rustFutureCompleteFunc[F],
	liftFunc func(F) T,
	rustFuture C.uint64_t,
	pollFunc rustFuturePollFunc,
	freeFunc rustFutureFreeFunc,
) (T, *E) {
	defer freeFunc(rustFuture)
	
	pollResult := int8(-1)
	waiter := make(chan int8, 1)

	chanHandle := cgo.NewHandle(waiter)
	defer chanHandle.Delete()

	for pollResult != uniffiRustFuturePollReady {
		pollFunc(
			rustFuture,
			(C.UniffiRustFutureContinuationCallback)(C.binary_options_tools_uni_uniffiFutureContinuationCallback),
			C.uint64_t(chanHandle),
		)
		pollResult = <-waiter
	}

	var goValue T
	var ffiValue F
	var err *E
	
	ffiValue, err = rustCallWithError(errConverter, func(status *C.RustCallStatus) F {
		return completeFunc(rustFuture, status)	
	})
	if err != nil {
		return goValue, err
	}
	return liftFunc(ffiValue), nil
}

//export binary_options_tools_uni_uniffiFreeGorutine
func binary_options_tools_uni_uniffiFreeGorutine(data C.uint64_t) {
	handle := cgo.Handle(uintptr(data))
	defer handle.Delete()

	guard := handle.Value().(chan struct{})
	guard <- struct{}{}
}

