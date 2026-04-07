package main

//export simpleLoop
func simpleLoop() {
	for {
	}
}

//export allocatingLoop
func allocatingLoop() {
	x := make([]int, 0)
	for i := 0; ; i++ {
		x = append(x, i)
	}
}

func main() {
}
