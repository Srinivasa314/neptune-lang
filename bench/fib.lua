function fib(n)
    if n < 2 then
        return n
    end
    return fib(n-1)+fib(n-2)
end

local time = os.clock()
print(fib(33))
print((os.clock()-time)*1000)