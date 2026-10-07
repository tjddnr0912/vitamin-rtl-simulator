typedef struct packed { logic valid; logic [2:0] tag; logic [3:0] rd; } pkt_t;
module top;
  localparam MSB = 1;
  pkt_t c;
  logic [1:0] r;
  initial begin
    c = '0;
    c.tag[MSB:0] = 2'b11;        // member part-select WRITE, range over a localparam
    r = c.tag[MSB:0];            // member part-select READ, range over a localparam
    #1 $display("c=%b r=%b", c, r);
    $finish;
  end
endmodule
