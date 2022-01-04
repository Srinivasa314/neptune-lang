function sieve(n)
    local sqrt_n = math.sqrt(n)
    local primes = {}
    for i =1,n do
        primes[i] = true
    end
    for p=2,sqrt_n do
        if(primes[p] == true) then
            for i = p*p,n,p do
                primes[i] = false
            end
        end
    end

    local count = 0
    for i = 2,n do
        if (primes[i] == true) then
            count = count + 1
        end
    end
    return count
end

local time = os.clock()
io.write(sieve(1000000))
io.write("\n",(os.clock()-time)*1000)
