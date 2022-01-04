def collatz(x) 
	while(x > 1)
		if(x % 2 == 0) then
			x=x/2
		else
			x = x * 3 + 1
        end
	end
end

for i in 1..100000 do
    collatz(i)
end
