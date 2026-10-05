module top;
  logic clk = 0, rst_n;
  logic [1:0] st;
  always #5 clk = ~clk;
  always_ff @(posedge clk or negedge rst_n)
    if (!rst_n) st <= 2'd0;
    else unique case (st) 2'd0: st <= 2'd1;
      2'd1: st <= 2'd2;
      2'd2: st <= 2'd0; endcase
  initial begin
    rst_n = 1;
    #12 rst_n = 0;
    #10 rst_n = 1;
    #40 $display("t=%0t st=%0d", $time, st);
    $finish;
  end
endmodule
