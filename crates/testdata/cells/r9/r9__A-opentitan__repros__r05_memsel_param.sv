typedef struct packed { logic [31:0] a; logic [3:0] m; } h_t;
module m #(parameter int AW = 8) (input h_t h, output logic [AW-1:0] o);
  assign o = {h.a[AW-1:2], 2'b00};
endmodule
module t;
  h_t h; logic [7:0] o;
  m #(.AW(8)) u (.h(h), .o(o));
  initial begin h = {32'h1234_56FF, 4'h3}; #1 $display("A o=%h", o); $finish; end
endmodule
