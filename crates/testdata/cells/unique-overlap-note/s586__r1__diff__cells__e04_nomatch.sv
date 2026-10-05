module top;
  logic [1:0] r = 2'b00; logic a = 0, b = 0; int y = 0;
  initial #100 $finish;
  initial begin
    #1;
    unique0 case (r) 2'b01: y = 1; 2'b10: y = 2; endcase
    priority0 if (a) y = 3; else if (b) y = 4;
    unique case (r) 2'b01: y = 5; 2'b10: y = 6; endcase
    priority if (a) y = 7; else if (b) y = 8;
    $display("y=%0d", y);
  end
endmodule
