sep = ARGV[0] || ""
words = STDIN.each_line.map(&:chomp).to_a
words.find_all{|w| w.size > 1}.repeated_combination(2) do |words|
    puts words.join sep
end