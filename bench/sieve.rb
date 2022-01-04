require 'benchmark'

def sieve(n)
    sqrt_n = Math.sqrt(n).to_i
    primes = Array.new(n+1,true)
    p = 2   
    while p <= sqrt_n
        if(primes[p] == true) then
            i = p * p
            while i <= n do
                primes[i] = false
                i += p
            end
        end        
        p += 1
    end

    count ,i = 0,2
    while i <= n
        if (primes[i] == true) then
            count += 1
        end
        i += 1
    end
    return count
end
time = Benchmark.measure {
puts sieve(1000000)
}

puts time.real*1000