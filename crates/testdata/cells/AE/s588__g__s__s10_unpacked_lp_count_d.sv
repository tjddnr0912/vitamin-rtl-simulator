module top;
  function automatic logic [3:0] fx(input int a);
    logic [3:0] t;
    case (a) 1: t = 4'd5; endcase
    fx = t;
  endfunction
  localparam int N = fx(2);
  logic [7:0] m [N];
  initial begin #2 $display("s=%0d", $size(m)); $finish; end
  initial #100 $finish;
endmodule
