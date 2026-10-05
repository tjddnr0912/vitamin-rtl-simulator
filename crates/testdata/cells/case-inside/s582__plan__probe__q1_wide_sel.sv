module top;
  logic [63:0] v;
  int m;
  task automatic t1(input logic [63:0] x);
    v = x;
    case (v) inside
      [0:3]: m = 1;
      32'sd7: m = 2;
      default: m = 0;
    endcase
    $display("A v=%h m=%0d", v, m);
  endtask
  task automatic t2(input logic [63:0] x);
    v = x;
    case (v) inside
      32'shFFFFFFFF: m = 3;
      default: m = 0;
    endcase
    $display("B v=%h m=%0d", v, m);
  endtask
  initial begin
    #10 $finish;
  end
  initial begin
    t1(64'd2); t1(64'd7); t1(64'd5); t1(64'hFFFFFFFF_FFFFFFFF); t1(64'h1_00000002);
    t2(64'h00000000_FFFFFFFF); t2(64'hFFFFFFFF_FFFFFFFF);
    $finish;
  end
endmodule
