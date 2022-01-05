function collatz(x) 
	while(x > 1)
    do 
		if(x % 2 == 0) then
			x=x/2
		else
			x = x * 3 + 1
        end
	end
end

local time = os.clock()
for i=1,99999 do
    collatz(i)
end
io.write("\n",(os.clock()-time)*1000)
