function sieve(n, primes)
    local sqrt_n = math.sqrt(n)
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

local primes = {}
for i =1,5000000 do
    primes[i] = true
end

local time = os.clock()
print(sieve(5000000, primes))
print((os.clock()-time)*1000)