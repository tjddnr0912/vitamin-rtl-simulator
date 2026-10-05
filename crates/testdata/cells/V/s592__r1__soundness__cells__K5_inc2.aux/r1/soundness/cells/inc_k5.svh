if (`V == 1) begin wire [3:0] w = 4'd1; initial #1 $display("@%m then w=%0d", w); end else begin wire [7:0] w = 8'd200; initial #1 $display("@%m else w=%0d", w); end
