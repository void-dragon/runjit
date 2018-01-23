# runjit

## Why?

Why another script language?

For some simple reason: Usabiliity

More easy integration. First i tried to use V8, but ran in compiling issues on Windows, then i tried to
use Lua but the stack centered approche lead to much boiler plate and it was combersome to call callback functions written in Lua.
I looked at the various other script languages for rust and found them, a littel ackward.
I wanted something straight forward and simple to use, because it is nice to get things done.
Well this is my try to make the world a bit simpler.

## Example

```js
// new variable
stuff = 1 + 2 + 3

// functions have no name, if you want lambdas with namse aka functions,
// then store them in variable, like any other variable
func = (a) => { print(a) }

func(stuff)

if 12 == 2 * 6 {
    print("should be 12")
} else {
    print("you never should see this")
}
```

## Additional Information

+ https://docs.rs/pest/1.0.0-beta.17/pest/
+ https://github.com/pest-parser/pest

+ http://llvm.org/docs/tutorial/LangImpl01.html
+ http://blog.ulysse.io/2016/07/03/llvm-getting-started.html
