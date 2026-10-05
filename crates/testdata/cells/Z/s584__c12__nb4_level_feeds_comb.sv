module top;
  wire [1:0] w;
  logic [1:0] p;
  wire [1:0] pv;
  logic [1:0] y;
  assign w = 2'd1;
  assign pv = p;
  always @* p = w;
  always_comb begin
    $display("C t=%0t pv=%b", $time, pv);
    y = 0;
    unique case (pv) 2'd1: y = 1; 2'd2: y = 2; endcase
  end
  final $display("F y=%0d", y);
endmodule
