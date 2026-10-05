module top;
  logic [1:0] r = 2'b00; int y;
  initial #100 $finish;
  initial begin
    #1;
`line 100 "renamed.sv" 0
    unique case (r)
      2'b01: y = 1;
      2'b10: y = 2;
    endcase
    $display("y=%0d", y);
  end
endmodule
