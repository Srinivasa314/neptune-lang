def Ack(m, n)
    if (m == 0) then
        return n + 1
    end
    if (n == 0) then
        return Ack(m - 1, 1)
    end
    return Ack(m - 1, Ack(m, (n - 1)))
end

print Ack(3,8)
