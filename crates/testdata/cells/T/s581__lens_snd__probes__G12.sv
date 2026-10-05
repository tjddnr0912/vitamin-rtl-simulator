module top;
  function automatic logic [1 + (4'bx100 ==? 4'b1?00):0] f(input int a); return a; endfunction
  initial $display("G12 %0d", $bits(f(3)));
endmodule
