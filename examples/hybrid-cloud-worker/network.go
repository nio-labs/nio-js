package main

/*
#include <stdlib.h>
*/
import "C"
import (
	"encoding/json"
	"math/rand"
    "unsafe"
)

//export FetchClusterStatus
func FetchClusterStatus() *C.char {
	// Simulate fetching status from a Kubernetes cluster or gRPC microservice
	status := map[string]interface{}{
		"active_nodes": 5,
		"status":       "healthy",
		"latency_ms":   rand.Intn(15) + 5,
	}
	
	bytes, _ := json.Marshal(status)
	return C.CString(string(bytes))
}

func main() {} // Required for cgo shared library

//export FreeCString
func FreeCString(value *C.char) { C.free(unsafe.Pointer(value)) }
