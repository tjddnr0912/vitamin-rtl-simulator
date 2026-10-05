package q;
  function automatic int f(input int a); return 3; endfunction
  parameter logic [f(2):0] P = 0;
endpackage
module top;
  function automatic int f(input int a);
    if (a == 1) return 1;
    return 7;
  endfunction
  initial begin $display("b=%0d", $bits(q::P)); #1 $finish; end
endmodule
