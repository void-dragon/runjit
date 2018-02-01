# runjit

## Why?

Why another script language?

For some simple reason: Usabiliity

More easy integration. First i tried to use V8, but ran in compiling issues on Windows, then i tried to
use Lua, but the stack centered approche lead to much boiler plate and it was combersome to call callback functions written in Lua.
I looked at the various other script languages for rust and found them, a littel ackward.
I wanted something straight forward and simple to use, because it is nice to get things done.
Well this is my try to make the world a little bit simpler.

## What?

And what is runjit?

Runjit is a single pass, jit compiled, script language.
Which means, that during the parsing of the source file the LLVM IR code is
directly generated and compiled direct after. Which has the nice feature that
you can directly pass functions callbacks between runjit and rust.

## Example

```js
// new variable
stuff = 1 + 2 + 3
anArray = [1, "2"]
aDict = { name: "the man" }

// functions have no name, if you want lambdas with namse aka functions,
// then store them in a variable, like any other variable
func = (a) => { print(a) }

func(stuff)

// ofcourse we have ifs
if 12 == 2 * 6 {
    print("should be 12")
} else {
    print("you never should see this")
}

// ... and loops too
x = 1
loop x < stuff {
  print(x)
  x += 1
}

```

## Additional Information

+ https://docs.rs/pest
+ https://github.com/pest-parser/pest

+ http://llvm.org/docs/tutorial/LangImpl01.html
+ http://blog.ulysse.io/2016/07/03/llvm-getting-started.html
+ https://pauladamsmith.com/blog/2015/01/how-to-get-started-with-llvm-c-api.html
