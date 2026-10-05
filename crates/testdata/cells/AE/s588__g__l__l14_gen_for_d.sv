module top;
  function automatic logic [3:0] fx(input int a);
    logic [3:0] t;
    case (a) 1: t = 4'd5; endcase
    fx = t;
  endfunction
  for (genvar i = 0; i < fx(2); i = i + 1) begin : g initial $display("I%0d", i); end
  initial begin #1 $display("done"); $finish; end
  initial #100 $finish;
endmodule
