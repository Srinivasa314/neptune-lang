require 'benchmark'

def sieve(primes)
    n = primes.length - 1 
    sqrt_n = Math.sqrt(n).to_i
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

primes = Array.new(5000001,true)

time = Benchmark.measure {
puts sieve(primes)
}

puts time.real*1000