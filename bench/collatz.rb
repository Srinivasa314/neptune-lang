require 'benchmark'

def collatz(x) 
	while(x > 1)
		if(x % 2 == 0) then
			x=x/2
		else
			x = x * 3 + 1
        end
	end
end

time = Benchmark.measure {
for i in 1..100000 do
    collatz(i)
end
}

puts time.real*1000