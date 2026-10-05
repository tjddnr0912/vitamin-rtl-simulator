module sub(input logic [f(1):0] p);
  function automatic int f(input int a);
    f = 7;
    unique if (a == 1) f = 10;
  endfunction
  initial begin #1 $display("pw=%0d", $bits(p)); $finish; end
endmodule
module top;
  logic [15:0] w;
  sub u(.p(w));
endmodule
