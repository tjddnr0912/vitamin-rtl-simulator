module top; wire [7:0] d = 8'd1; logic [7:0] e;
always_comb begin e = d ^ 8'hA5; $display("C %0t d=%0d e=%0d", $time, d, e); end
initial #3 $finish;
endmodule
