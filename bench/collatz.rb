require 'benchmark'

$max_steps = 0

def collatz(x) 
	steps = 0
	while(x > 1)
		steps+=1
		if(x % 2 == 0) then
			x=x/2
		else
			x = x * 3 + 1
        end
	end
	if steps > $max_steps then
		$max_steps=steps
	end
end

time = Benchmark.measure {
for i in 1..100000 do
    collatz(i)
end
}
puts $max_steps
puts time.real*1000