module top;
  function automatic logic [3:0] fx(input int a);
    logic [3:0] t;
    case (a) 1: t = 4'd5; endcase
    fx = t;
  endfunction
  function automatic int h(input logic [fx(2):0] b);
    h = $bits(b);
  endfunction
  localparam int P = h(0);
  initial begin #2 $display("P=%0d", P); $finish; end
  initial #100 $finish;
endmodule
