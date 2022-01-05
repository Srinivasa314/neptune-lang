local max_steps = 0

function collatz(x) 
	local steps = 0
	while(x > 1)
    do 
		steps=steps+1
		if(x % 2 == 0) then
			x=x/2
		else
			x = x * 3 + 1
        end
	end
	if steps > max_steps then
		max_steps=steps
	end
end

local time = os.clock()
for i=1,99999 do
    collatz(i)
end
print(max_steps)
print((os.clock()-time)*1000)