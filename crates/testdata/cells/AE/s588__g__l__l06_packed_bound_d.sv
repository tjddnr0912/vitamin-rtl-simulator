module top;
  function automatic logic [3:0] fx(input int a);
    logic [3:0] t;
    case (a) 1: t = 4'd5; endcase
    fx = t;
  endfunction
  logic [fx(2):0] v;
  initial begin #1 $display("b=%0d", $bits(v)); $finish; end
  initial #100 $finish;
endmodule
