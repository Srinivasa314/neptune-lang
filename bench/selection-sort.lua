function sortDescending(arr)
    local len = #arr
    for i=1,len do
        for j=i+1,len do
            if (arr[i] < arr[j]) then
                local t = arr[i]
                arr[i] = arr[j]
                arr[j] = t
            end
        end
    end
end

local time = os.clock()
local arr = {}
for i=1,4000 do
    arr[i] = i-1
end
sortDescending(arr)
print(arr[1])
print((os.clock()-time)*1000)