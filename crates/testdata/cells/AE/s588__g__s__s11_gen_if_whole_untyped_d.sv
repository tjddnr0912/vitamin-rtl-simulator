module top;
  function automatic logic [3:0] fx(input int a);
    logic [3:0] t;
    case (a) 1: t = 4'd5; endcase
    fx = t;
  endfunction
  localparam X = fx(2);
  if (X) begin : g1 initial $display("T"); end else begin : g2 initial $display("E"); end
  initial begin #2 $display("done"); $finish; end
  initial #100 $finish;
endmodule
