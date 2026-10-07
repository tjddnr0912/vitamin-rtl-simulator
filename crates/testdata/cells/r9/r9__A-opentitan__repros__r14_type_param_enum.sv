module ff #(parameter int W = 2, parameter type T = logic [W-1:0], parameter T R = T'(0)) (input logic clk, input logic rst_n, input T d, output T q);
  always_ff @(posedge clk or negedge rst_n) if (!rst_n) q <= R; else q <= d;
endmodule
module t;
  typedef enum logic [1:0] { Idle = 2'b01, Run = 2'b10, Done = 2'b11 } st_e;
  logic clk = 0, rst_n = 0; st_e d, q;
  ff #(.W(2), .T(st_e), .R(Idle)) u (.clk(clk), .rst_n(rst_n), .d(d), .q(q));
  always #5 clk = ~clk;
  initial begin d = Run; #1 $display("A q=%s %b", q.name(), q); rst_n = 1; #10 $display("A q=%s %b", q.name(), q); d = Done; #10 $display("A q=%s %b", q.name(), q); $finish; end
endmodule
