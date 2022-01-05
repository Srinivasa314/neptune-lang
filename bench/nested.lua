function loop()
    local counter = 0
    for a = 1,75 do
        for b = 1,75 do
            for c = 1,75 do
                for d = 1,75 do
                    counter = counter + 1
                end
            end
        end
    end    
    return counter               
end

local time = os.clock()
print(loop())
print((os.clock()-time)*1000)