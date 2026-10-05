module top;
  logic a = 1'b0, b = 1'b1; logic [1:0] y, m, z;
  function logic [1:0] f(input logic x, input logic w);
    $display("f t=%0t x=%b w=%b", $time, x, w);
    return {x, w};
  endfunction
  always_comb begin
    z = 0;
    unique case (m) 2'b01: z = 1; 2'b10: z = 2; endcase
  end
  always_comb m = y;
  assign y = f(a, b);
  initial begin
    #1 $display("t=%0t y=%b m=%b z=%b", $time, y, m, z);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
