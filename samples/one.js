outer = (1 + 2) + 3

bugy = (a) => { print(a) }

bugy("mauz")

bugy(outer)

bugy = (a) => {
    print(outer, a)
}

bugy("mauz again")

if outer == 6 {
    print("yes indeed it is six")
} else {
    print("you never should see this")
}

nine = (2 + 1) * 3
five = 2 + (1 * 3)

if nine != five {
    print("indeed: ", nine, "!=", five)
}

myDict = {
    stuff: "marvin"
}

myArr = [1, "moaw"]

print("dic", myDict)
print("arr", myArr)
