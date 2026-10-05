module top; logic [7:0] c = 0; wire [7:0] d; logic [7:0] e; assign d = c + 8'd1;
always_comb begin e = d ^ 8'hA5; $display("C %0t d=%0d e=%0d", $time, d, e); end
initial #3 $finish;
endmodule
