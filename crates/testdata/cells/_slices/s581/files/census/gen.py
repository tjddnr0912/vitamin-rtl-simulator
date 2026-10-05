#!/usr/bin/env python3
# Generate generate-case census cells: one design per cell, one generate-case per design
# (unless a cell is raw). Each arm prints "<ID> <arm>".
import os, sys, json
D = os.path.dirname(os.path.abspath(__file__)) + '/cells'
os.makedirs(D, exist_ok=True)
cells = []  # (id, scrut, label-desc, text)

def arm(cid, name, labels):
    head = 'default' if labels is None else labels
    return f'      {head}: begin : g_{name} initial $display("{cid} {name}"); end'

def std(cid, S, items, decls='', default=True, pre_items=None, desc=None):
    lines = ['module top;']
    if decls:
        lines += ['  ' + d for d in decls.strip().split('\n')]
    lines += ['  generate', f'    case ({S})']
    if pre_items:
        lines += [arm(cid, n, l) for (l, n) in pre_items]
    lines += [arm(cid, n, l) for (l, n) in items]
    if default:
        lines.append(arm(cid, 'def', None))
    lines += ['    endcase', '  endgenerate', 'endmodule', '']
    lab = ' | '.join(l if l is not None else 'default' for (l, n) in (pre_items or []) + items)
    cells.append((cid, S, desc or lab, '\n'.join(lines)))

def raw(cid, S, desc, text):
    cells.append((cid, S, desc, text.strip() + '\n'))

# ---- L: label kinds, known scrutinee ----
std('L01', '2', [('1', 'a'), ('2', 'b')])
std('L02', '3', [('P', 'a')], 'localparam P = 3;')
std('L03', '4', [('P + 1', 'a')], 'localparam P = 3;')
std('L04', 'P16', [('"ab"', 'a')], "localparam logic [15:0] P16 = 16'h6162;")
std('L05', "16'h6162", [('S1', 'a')], 'localparam S1 = "ab";')
std('L06', 'P', [('"a" + 1', 'a')], "localparam logic [7:0] P = 8'h62;")
std('L07', "16'h6163", [('S1 + 1', 'a')], 'localparam S1 = "ab";')
std('L08', "16'h6162", [('SS', 'a')], 'parameter string SS = "ab";')
std('L09', '1', [('$clog2(2)', 'a')])
std('L10', '3', [('f(2)', 'a')], 'function automatic integer f(input integer x); f = x + 1; endfunction')
std('L11', '2', [('E2', 'a')], "typedef enum logic [1:0] {E0, E1, E2} e_t;")
std('L12', '1', [('1.0', 'a')])
std('L13', '1', [('R', 'a')], 'localparam real R = 1.0;')
std('L14', '3', [('top.P', 'a')], 'localparam P = 3;')
std('L15', '1', [('w', 'a')], "wire [3:0] w = 4'd1;")
raw('L16', '5', 'pk::K (8-bit pkg const)', '''
package pk; localparam logic [7:0] K = 8'd5; endpackage
module top;
  generate
    case (5)
      pk::K: begin : g_a initial $display("L16 a"); end
      default: begin : g_def initial $display("L16 def"); end
    endcase
  endgenerate
endmodule''')
std('L17', '3', [('W', 'a')], 'localparam [64:0] W = 65\'d3;')
std('L18', '1', [('W', 'a')], "localparam [64:0] W = {1'b1, 64'd1};")
std('L19', '1', [("65'h1_0000_0000_0000_0001", 'a')])
std('L20', '1', [("65'd1", 'a')])
std('L21', '15', [('P4', 'a')], "localparam logic [3:0] P4 = 4'b1111;")
std('L22', '-1', [('PS', 'a')], 'localparam logic signed [3:0] PS = -1;')
std('L23', '15', [('PS', 'a')], 'localparam logic signed [3:0] PS = -1;')
std('L24', "4'd15", [('PS', 'a')], 'localparam logic signed [3:0] PS = -1;')
std('L25', '1', [('g()', 'a')], "function [64:0] g(); g = 65'd1; endfunction")
std('L26', "8'h62", [('"a" + 1', 'a')])
std('L27', '2', [('$bits(2\'b00)', 'a')])
std('L28', '1', [("P > 0", 'a')], 'localparam [64:0] P = {1\'b1, 64\'d0};')
std('L29', '0', [("W >> 64", 'a'), ('0', 'b')], "localparam [64:0] W = {1'b1, 64'd0};", desc='W>>64 (=1) | 0')
std('L30', '1', [("W >> 64", 'a')], "localparam [64:0] W = {1'b1, 64'd0};")
# t1..t4, c1..c3 from the brief
std('T1', '1', [('LPA', 'a')], "localparam P1 = (4'b1100 ==? 4'b1?00);\nlocalparam [64:0] LPA = {64'd0, P1};")
std('T2', '1', [("$isunknown(4'bx100 ==? 4'b1?00)", 'a')])
std('T3', '0', [("(4'bx100 ==? 4'b1?00) >> 1", 'a')])
std('T4', '1', [("4'b1100 inside {{2'b1?, 2'b00}}", 'a')])
std('C1', '1', [("(4'bx100 ==? 4'b1?00)", 'a')])
std('C2', '1', [("65'd1", 'a')])
std('C3', '1', [("4'bx", 'a')])
# ---- X: x/z labels ----
std('X01', "4'b1100", [("4'b1x00", 'a')])
std('X02', '1', [("1'bx", 'a'), ('1', 'b')])
std('X03', '1', [("4'bz", 'a')])
std('X04', '4', [("4'b1z00", 'a')])
std('X05', '0', [("4'b1x00 & 4'b0011", 'a')])
std('X06', '1', [("4'b1x00 == 4'b1100", 'a')])
std('X07', '0', [("4'b1x00 == 4'b0000", 'a')])
std('X08', '1', [("4'b1x00 === 4'b1x00", 'a')])
std('X09', '1', [("$isunknown(4'b1x00)", 'a')])
std('X10', '1', [("|4'b100x", 'a')])
std('X11', '1', [("(4'b1100 ==? 4'b1?00)", 'a')])
std('X12', '0', [("(4'b1100 ==? 4'b0?00)", 'a')])
std('X13', '1', [("4'b1100 inside {4'b1?00}", 'a')])
std('X14', '1', [("4'b1100 inside {4'b1100, 4'b0000}", 'a')])
std('X15', "4'b1100", [('PX', 'a')], "localparam logic [3:0] PX = 4'b1x00;")
std('X16', '0', [("|4'b000x", 'a')])
std('X17', '1', [("{64'd0, 1'bx}", 'a'), ('1', 'b')])
std('X18', '1', [("65'bx", 'a'), ('1', 'b')])
std('X19', "4'b1100", [("{2'b11, 2'bx0} | 4'b0001", 'a'), ("4'b1100", 'b')])
std('X20', '1', [("1'bx ? 1 : 1", 'a')])
std('X21', '1', [("(4'bx100 !=? 4'b1?00)", 'a')])
std('X22', '0', [("(4'b1100 ==? 4'b0?00) >> 1", 'a')])
std('X23', '1', [("4'b1100 inside {4'b11x0}", 'a')])
# ---- S: scrutinee kinds and the §12.5 width/sign axis ----
std('S01', "1'bx", [("1'bx", 'a')])
std('S02', "65'd1", [('1', 'a')])
std('S03', 'W', [('W', 'a')], "localparam [64:0] W = {1'b1, 64'd0};")
std('S04', '"ab"', [("16'h6162", 'a')])
std('S05', '-1', [('-1', 'a')])
std('S06', '-1', [("32'hFFFFFFFF", 'a')])
std('S07', "4'sb1111", [("4'b1111", 'a')])
std('S08', "4'sb1111", [('-1', 'a')])
std('S09', "4'b1111", [('-1', 'a')])
std('S10', "4'sb1111", [("8'd255", 'a')])
std('S11', "4'sb1111", [("8'd15", 'a')])
std('S12', "8'sb11111111", [("4'sb1111", 'a')])
std('S13', "64'hFFFF_FFFF_FFFF_FFFF", [('-1', 'a')])
std('S14', "64'hFFFF_FFFF_FFFF_FFFF", [("-64'sd1", 'a')])
std('S15', '-1', [("64'hFFFF_FFFF_FFFF_FFFF", 'a')])
std('S16', "32'hFFFFFFFF", [('-1', 'a')])
std('S17', "33'h1FFFFFFFF", [('-1', 'a')])
std('S18', "-33'sd1", [('-1', 'a')])
std('S19', "65'h1FFFFFFFFFFFFFFFF", [('-1', 'a')])
std('S20', "4'sb1000", [('-8', 'a')])
std('S21', "4'sb1000", [("4'd8", 'a')])
std('S22', "4'sb1000", [('8', 'a')])
std('S23', "4'd8", [("4'sb1000", 'a')])
std('S24', "4'd15", [("8'sb11111111", 'a')])
std('S25', "4'd15", [("8'sb00001111", 'a')])
std('S26', '-1', [("33'h1FFFFFFFF", 'a')])
std('S27', '-1', [("33'h0FFFFFFFF", 'a')])
std('S28', "$signed(4'b1111)", [("4'b1111", 'a')])
std('S29', 'PI', [("32'hFFFFFFFF", 'a')], 'localparam int PI = -1;')
std('S30', 'UP', [("4'b1111", 'a')], "localparam UP = 4'sb1111;")
std('S31', 'PU', [('-1', 'a')], "localparam logic [7:0] PU = 8'hFF;")
std('S32', 'PS8', [("8'hFF", 'a')], 'localparam logic signed [7:0] PS8 = -1;')
std('S33', "64'hFFFF_FFFF_FFFF_FFFF", [("64'hFFFF_FFFF_FFFF_FFFF", 'a')])
std('S34', "-64'sd1", [("65'h1FFFFFFFFFFFFFFFF", 'a')])
std('S35', "-64'sd1", [("-65'sd1", 'a')])
std('S36', "4'b1111", [('UP', 'a')], "localparam UP = 4'sb1111;")
std('S37', "4'b1111", [('PS', 'a')], 'localparam logic signed [3:0] PS = -1;')
std('S38', '-1', [('PI', 'a')], "localparam int PI = -1;")
std('S39', "32'hFFFFFFFF", [('PI', 'a')], "localparam int PI = -1;")
std('S40', "8'sd127", [("8'sd127", 'a')])
std('S41', 'P', [('-1', 'a')], 'localparam integer P = -1;')
std('S42', 'f(2)', [('3', 'a')], 'function automatic integer f(input integer x); f = x + 1; endfunction')
std('S43', 'f(2)', [("65'd3", 'a')], 'function automatic integer f(input integer x); f = x + 1; endfunction')
std('S44', 'f(2)', [("$isunknown(4'bx100 ==? 4'b1?00)", 'a'), ('3', 'b')], 'function automatic integer f(input integer x); f = x + 1; endfunction')
std('S45', 'UP', [("4'sb1111", 'a')], "localparam UP = 4'sb1111;")
std('S46', 'U8', [('-1', 'a')], "localparam U8 = 8'hFF;")
std('S47', 'f(2)', [("{64'd0, 1'bx}", 'a'), ('3', 'b')], 'function automatic integer f(input integer x); f = x + 1; endfunction')
# ---- O: override channel (sub-module parameter as scrutinee) ----
def osub(cid, pdecl, S, lab, ov):
    raw(cid, S, f'{pdecl} ov={ov} | {lab}', f'''
module sub #({pdecl});
  generate
    case ({S})
      {lab}: begin : g_a initial $display("{cid} a"); end
      default: begin : g_def initial $display("{cid} def"); end
    endcase
  endgenerate
endmodule
module top;
  sub #(.P({ov})) u();
endmodule''')
osub('O01', 'parameter int P = 0', 'P', "32'hFFFFFFFF", '-1')
osub('O02', 'parameter logic signed [3:0] P = 0', 'P', "4'b1111", '-1')
osub('O03', 'parameter P = 0', 'P', "4'b1111", "4'sb1111")
osub('O04', 'parameter logic [7:0] P = 0', 'P', '-1', '-1')
osub('O05', 'parameter P = 0', 'P', '-1', "8'hFF")
osub('O06', 'parameter [64:0] P = 0', 'P', "65'd1", '1')
osub('O07', 'parameter int P = 0', "4'd3", 'P', '3')
osub('O08', 'parameter logic [64:0] P = 0', "1", 'P', "{1'b1, 64'd1}")
osub('O09', 'parameter logic [64:0] P = 0', "1", 'P', "65'd1")
osub('O10', 'parameter P = 0', "1", 'P', "65'd1")
# ---- M: several labels / items, first match ----
std('M01', '2', [('1, 2', 'a'), ('2', 'b')])
std('M02', '3', [('1', 'a'), ('3', 'b'), ('3', 'c')])
std('M03', '1', [('1', 'a')], pre_items=[(None, 'def0')], default=False)
std('M04', '1', [("$isunknown(4'bx100 ==? 4'b1?00)", 'a'), ('1', 'b')])
std('M05', '1', [('1', 'a'), ("$isunknown(4'bx100 ==? 4'b1?00)", 'b')])
std('M06', '0', [("$isunknown(4'bx100 ==? 4'b1?00)", 'a'), ('0', 'b')])
std('M07', '1', [("1'bx, 1", 'a')])
std('M08', '1', [("65'd2, LPA", 'a')], "localparam P1 = (4'b1100 ==? 4'b1?00);\nlocalparam [64:0] LPA = {64'd0, P1};")
std('M09', '1', [('2', 'a')], default=False)
std('M10', '0', [("(4'bx100 ==? 4'b1?00), 0", 'a')])
std('M11', '1', [('1.0, 1', 'a')])
std('M12', '1', [('w, 1', 'a')], "wire [3:0] w = 4'd1;")
# ---- N: nested scopes ----
raw('N01', 'genvar i', 'for i 0..2: case(i) 0/1/def', '''
module top;
  for (genvar i = 0; i < 3; i++) begin : g
    case (i)
      0: begin : c0 initial $display("N01 c0_%0d", i); end
      65'd1: begin : c1 initial $display("N01 c1_%0d", i); end
      default: begin : cd initial $display("N01 cd_%0d", i); end
    endcase
  end
endmodule''')
raw('N02', '2', 'for i: localparam [64:0] L = 65\'d1 + i; case(2) L', '''
module top;
  for (genvar i = 0; i < 3; i++) begin : g
    localparam [64:0] L = 65'd1 + i;
    case (2)
      L: begin : c0 initial $display("N02 a_%0d", i); end
      default: begin : cd initial $display("N02 def_%0d", i); end
    endcase
  end
endmodule''')
raw('N03', '7', 'shadow: outer [64:0] L=5, inner [64:0] L=7', '''
module top;
  localparam [64:0] L = 65'd5;
  if (1) begin : gb
    localparam [64:0] L = 65'd7;
    case (7)
      L: begin : c0 initial $display("N03 a"); end
      default: begin : cd initial $display("N03 def"); end
    endcase
  end
endmodule''')
raw('N04', '7', 'shadow: outer [64:0] L=5, inner logic[3:0] L=7', '''
module top;
  localparam [64:0] L = 65'd5;
  if (1) begin : gb
    localparam logic [3:0] L = 4'd7;
    case (7)
      L: begin : c0 initial $display("N04 a"); end
      default: begin : cd initial $display("N04 def"); end
    endcase
  end
endmodule''')
raw('N05', '5', 'shadow: outer logic[3:0] L=5, inner [64:0] L=7', '''
module top;
  localparam logic [3:0] L = 4'd5;
  if (1) begin : gb
    localparam [64:0] L = 65'd7;
    case (5)
      L: begin : c0 initial $display("N05 a"); end
      default: begin : cd initial $display("N05 def"); end
    endcase
  end
endmodule''')
raw('N06', '2 (nested case)', 'case(1) 1: case(2) 65\'d2', '''
module top;
  case (1)
    1: begin : o
      case (2)
        65'd2: begin : c0 initial $display("N06 a"); end
        default: begin : cd initial $display("N06 def"); end
      endcase
    end
    default: begin : od initial $display("N06 odef"); end
  endcase
endmodule''')
raw('N07', '1', 'for i: localparam [64:0] LG = i; case(1) LG', '''
module top;
  for (genvar i = 0; i < 3; i++) begin : g
    localparam [64:0] LG = i;
    case (1)
      LG: begin : c0 initial $display("N07 a_%0d", i); end
      default: begin : cd initial $display("N07 def_%0d", i); end
    endcase
  end
endmodule''')
raw('N08', 'i', 'for i: case(i) {64\'d0,1\'b1}', '''
module top;
  for (genvar i = 0; i < 3; i++) begin : g
    case (i)
      {64'd0, 1'b1}: begin : c0 initial $display("N08 a_%0d", i); end
      default: begin : cd initial $display("N08 def_%0d", i); end
    endcase
  end
endmodule''')
raw('N09', 'i', 'for i: case(i) -1 / 32\'hFFFFFFFF', '''
module top;
  for (genvar i = 0; i < 2; i++) begin : g
    case (i - 1)
      32'hFFFFFFFF: begin : c0 initial $display("N09 a_%0d", i); end
      default: begin : cd initial $display("N09 def_%0d", i); end
    endcase
  end
endmodule''')
raw('N10', 'P (module param in gen block)', 'gen-if: localparam logic signed [3:0] Q=-1; case(Q) 4\'b1111', '''
module top;
  if (1) begin : gb
    localparam logic signed [3:0] Q = -1;
    case (Q)
      4'b1111: begin : c0 initial $display("N10 a"); end
      default: begin : cd initial $display("N10 def"); end
    endcase
  end
endmodule''')
raw('N11', 'i', 'for i: localparam logic [3:0] Q = i; case(Q) -1+i... label i', '''
module top;
  for (genvar i = 0; i < 2; i++) begin : g
    localparam logic signed [3:0] Q = i - 1;
    case (Q)
      4'b1111: begin : c0 initial $display("N11 a_%0d", i); end
      default: begin : cd initial $display("N11 def_%0d", i); end
    endcase
  end
endmodule''')
# ---- Z: casez / casex in a generate region ----
raw('Z01', '1', 'casez in generate', '''
module top;
  generate
    casez (1)
      1: begin : g_a initial $display("Z01 a"); end
      default: begin : g_def initial $display("Z01 def"); end
    endcase
  endgenerate
endmodule''')
raw('Z02', '1', 'casex in generate', '''
module top;
  generate
    casex (1)
      1: begin : g_a initial $display("Z02 a"); end
      default: begin : g_def initial $display("Z02 def"); end
    endcase
  endgenerate
endmodule''')

# ---- W: one width and sign for the whole case (§12.5), context-determined items ----
std('W01', "4'd15 + 4'd1", [('0', 'a'), ('16', 'b')])
std('W02', "4'd15 + 4'd1", [("4'd0", 'a'), ("5'd16", 'b')])
std('W03', "4'd15 + 4'd1", [("4'd0", 'a')])
std('W04', "4'sb1111", [('-1', 'a'), ("8'd0", 'b')])
std('W05', "4'sb1111", [('-1', 'a'), ("8'sd0", 'b')])
std('W06', "4'sb1111", [("4'sb1111", 'a'), ("8'd0", 'b')])
std('W07', "4'sb1000", [("8'sb11111000", 'a'), ("4'd0", 'b')])
std('W08', "5'd16", [("4'd15 + 4'd1", 'a')])
std('W09', "8'd0", [("4'd15 + 4'd1", 'a')])
std('W10', "P4 + 4'd1", [('0', 'a'), ('16', 'b')], "localparam logic [3:0] P4 = 4'd15;")
std('W11', "P4 + 4'd1", [("4'd0", 'a')], "localparam logic [3:0] P4 = 4'd15;")
std('W12', "4'd15 + 4'd1", [("4'd0", 'a'), ("f(1)", 'b')], 'function automatic integer f(input integer x); f = x; endfunction')
std('W13', "4'sb1111", [("8'sb11111111", 'a'), ("4'd0", 'b')])
std('W14', "4'sb1111", [("-1", 'a')], pre_items=[("4'd2", 'z')])
std('W15', "~4'd0", [("8'hFF", 'a'), ("4'hF", 'b')])
std('W16', "~4'd0", [("4'hF", 'a')])
std('W17', "4'd8 >> 1", [("5'd4", 'a')])
std('W18', "4'd8 << 1", [("5'd16", 'a'), ("4'd0", 'b')])
std('W19', "4'd8 << 1", [("4'd0", 'a')])
std('W20', "8'd200 + 8'd100", [("9'd300", 'a'), ("8'd44", 'b')])
# ---- E: enum-typed parameters and labels ----
std('E01', 'P', [('A', 'a'), ('B', 'b'), ('C', 'c')], "typedef enum logic [1:0] {A, B, C} e_t;\nparameter e_t P = C;")
std('E02', 'P', [('A', 'a'), ('B', 'b')], "typedef enum {A = -1, B = 3} e_t;\nparameter e_t P = A;")
std('E03', 'P', [("32'hFFFFFFFF", 'a'), ('B', 'b')], "typedef enum {A = -1, B = 3} e_t;\nparameter e_t P = A;")
std('E04', "2'b10", [('C', 'c'), ('A', 'a')], "typedef enum logic [1:0] {A, B, C} e_t;")
std('E05', 'P', [("4'b1111", 'a')], "typedef enum logic signed [3:0] {M = -1, Z = 0} e_t;\nparameter e_t P = M;")
std('E06', "4'b1111", [('M', 'a')], "typedef enum logic signed [3:0] {M = -1, Z = 0} e_t;")
# ---- F: fill and unsized labels ----
std('F01', "4'b1111", [("'1", 'a')])
std('F02', "8'hFF", [("'1", 'a')])
std('F03', "4'b0000", [("'0", 'a')])
std('F04', "65'h1_FFFF_FFFF_FFFF_FFFF", [("'1", 'a')])
std('F05', "4'b1111", [("'1", 'a'), ("8'd0", 'b')])
std('F06', "4'b1x00", [("4'b1x00", 'a')])
std('F07', "'1", [("4'b1111", 'a')])
std('F08', "4'sb1111", [("8'sd255", 'a')])
std('F09', "8'sd255", [('-1', 'a')])
# ---- R: untyped-parameter scrutinee (no declared width) ----
std('R01', 'M', [('LPA', 'a')], "localparam M = 1;\nlocalparam [64:0] LPA = 65'd1;")
std('R02', 'M', [('LPA', 'a'), ('1', 'b')], "localparam M = 1;\nlocalparam [64:0] LPA = 65'd2;")
std('R03', 'M', [('0', 'a'), ('1', 'b')], "localparam M = 1;")
std('R04', 'M', [("32'hFFFFFFFF", 'a')], "localparam M = -1;")
std('R05', 'M', [('"a" + 1', 'a')], "localparam M = 8'h62;")
std('R06', "P0 - 1", [("32'hFFFFFFFF", 'a')], "localparam P0 = 0;")
std('R07', "PI0 - 1", [("32'hFFFFFFFF", 'a')], "localparam int PI0 = 0;")
std('R08', 'M', [("$isunknown(4'bx100 ==? 4'b1?00)", 'a'), ('1', 'b')], "localparam M = 1;")

# ---- O2: untyped-parameter override channel (§6.20.2: the final value's type) ----
osub('O11', 'parameter P = 0', 'P', "32'hFFFFFFFF", '-1')
osub('O12', "parameter P = 4'd0", 'P', "32'hFFFFFFFF", '-1')
osub('O13', 'parameter P = 0', 'P', "8'hFF", "8'hFF")
osub('O14', "parameter P = -1", 'P', "4'b1111", "-1")
osub('O15', "parameter P = 4'sb1111", 'P', '-1', "4'd15")
osub('O16', "parameter P = 4'd15", 'P', '-1', "4'sb1111")
osub('O19', "parameter P = 8'd0", 'P', '-1', "4'sb1111")
osub('O20', "parameter P = 4'sb1111", "4'b1111", 'P', "4'd15")
osub('O21', "parameter P = 4'd15", "4'b1111", 'P', "4'sb1111")
osub('O22', "parameter P = 0", '1', 'P', "65'd1")
osub('O23', "parameter P = 0", "P", "65'h1_0000_0000_0000_0001", "65'h1_0000_0000_0000_0001")
raw('O17', 'P (defparam)', "parameter P = 4'd0; defparam u.P = 4'sb1111 | -1", """
module sub #(parameter P = 4'd0);
  generate
    case (P)
      -1: begin : g_a initial $display("O17 a"); end
      default: begin : g_def initial $display("O17 def"); end
    endcase
  endgenerate
endmodule
module top;
  sub u();
  defparam u.P = 4'sb1111;
endmodule""")
raw('O18', 'P (defparam)', "parameter logic [7:0] P = 0; defparam u.P = -1 | 8'hFF", """
module sub #(parameter logic [7:0] P = 0);
  generate
    case (P)
      8'hFF: begin : g_a initial $display("O18 a"); end
      default: begin : g_def initial $display("O18 def"); end
    endcase
  endgenerate
endmodule
module top;
  sub u();
  defparam u.P = -1;
endmodule""")
std('Q01', 'S1', [('"ab"', 'a')], 'localparam S1 = "ab";')
std('Q02', '16\'h6162', [('S1', 'a'), ('"ab"', 'b')], 'localparam string S1 = "ab";')
std('Q03', 'P', [('"ab"', 'a'), ('"c"', 'c')], "localparam logic [15:0] P = \"c\";")
std('Q04', "8'h63", [('"c"', 'c'), ('"ab"', 'a')])
std('Q05', "24'h006263", [('"bc"', 'a')])
std('Q06', "16'h6263", [('"\\0bc"', 'a')])

std('S48', 'f(2)', [('W', 'a')], "function automatic integer f(input integer x); f = x + 1; endfunction\nlocalparam [64:0] W = {1'b1, 64'd0};")

# ---- P: package / $unit / imported names (the other binders) ----
def pkcell(cid, pkg, pre, S, lab, desc):
    raw(cid, S, desc, f"""
{pkg}
{pre}
module top;
  generate
    case ({S})
      {lab}: begin : g_a initial $display("{cid} a"); end
      default: begin : g_def initial $display("{cid} def"); end
    endcase
  endgenerate
endmodule""")
pkcell('P01', "package pk; localparam logic signed [3:0] KS = -1; endpackage", '', "4'b1111", 'pk::KS', "pk::KS signed 4-bit -1")
pkcell('P02', "package pk; localparam logic signed [3:0] KS = -1; endpackage", 'import pk::*;', "4'b1111", 'KS', "imported KS signed 4-bit -1")
pkcell('P03', "localparam logic signed [3:0] US = -1;", '', "4'b1111", 'US', "$unit US signed 4-bit -1")
pkcell('P04', "package pk; localparam [64:0] KW = {1'b1, 64'd0}; endpackage", '', "1", 'pk::KW >> 64', "pk::KW 65-bit >> 64")
pkcell('P05', "package pk; localparam [64:0] KW = {1'b1, 64'd0}; endpackage", 'import pk::*;', "1", 'KW >> 64', "imported KW 65-bit >> 64")
pkcell('P06', "package pk; localparam KU = 4'sb1111; endpackage", '', "4'b1111", 'pk::KU', "pk::KU untyped 4'sb1111")

# ---- G: a genvar shadowing a same-named module constant ----
raw('G01', '1', 'module [64:0] i=9; for genvar i: case(1) i', """
module top;
  localparam [64:0] i = 65'd9;
  for (genvar i = 0; i < 2; i++) begin : g
    case (1)
      i: begin : c0 initial $display("G01 a_%0d", i); end
      default: begin : cd initial $display("G01 def_%0d", i); end
    endcase
  end
endmodule""")
raw('G02', 'i', 'module [64:0] i=9; for genvar i: case(i) 65 bit 9 / 1', """
module top;
  localparam [64:0] i = 65'd9;
  for (genvar i = 0; i < 2; i++) begin : g
    case (i)
      65'd9: begin : c0 initial $display("G02 nine_%0d", i); end
      65'd1: begin : c1 initial $display("G02 one_%0d", i); end
      default: begin : cd initial $display("G02 def_%0d", i); end
    endcase
  end
endmodule""")
raw('G03', '1', 'module [64:0] W; gen block localparam W narrow; case(1) W >> 0', """
module top;
  localparam [64:0] W = {1'b1, 64'd0};
  if (1) begin : gb
    localparam logic [3:0] W = 4'd1;
    case (1)
      W >> 0: begin : c0 initial $display("G03 a"); end
      default: begin : cd initial $display("G03 def"); end
    endcase
  end
endmodule""")
raw('G04', 'i', 'module signed [3:0] i=-1; for genvar i: case(i) 4b1111', """
module top;
  localparam logic signed [3:0] i = -1;
  for (genvar i = 0; i < 2; i++) begin : g
    case (i)
      4'b1111: begin : c0 initial $display("G04 a_%0d", i); end
      default: begin : cd initial $display("G04 def_%0d", i); end
    endcase
  end
endmodule""")

# ---- NA: name axis (generate-position name resolution), label AND scrutinee position.
# Every arm declares and drives a net and prints it at #1: an arm chosen in one phase
# and not another prints x (or nothing).
def A(cid, arm, lab, k):
    return f"{lab}: begin : g_{arm} wire [7:0] w = 8'd{k}; initial #1 $display(\"{cid} {arm} %0d\", w); end"
def DF(cid, k=99):
    return A(cid, 'def', 'default', k)
def na(cid, desc, text):
    raw(cid, desc.split('|')[0], desc, text)
SH = {  # (outer decl, inner decl) — outer value 5, inner value 7
 '1': ("localparam logic [3:0] K = 4'd5;", "localparam logic [3:0] K = 4'd7;", 'narrow/narrow'),
 '2': ("localparam [64:0] K = 65'd5;", "localparam [64:0] K = 65'd7;", 'wide/wide'),
 '3': ("localparam [64:0] K = 65'd5;", "localparam logic [3:0] K = 4'd7;", 'narrow inner over wide outer'),
 '4': ("localparam logic [3:0] K = 4'd5;", "localparam [64:0] K = 65'd7;", 'wide inner over narrow outer'),
}
for k, (o, i, d) in SH.items():
    c = f'NA0{k}L'
    na(c, f'7|label K, gen-if shadow {d}', f"""
module top;
  {o}
  if (1) begin : gb
    {i}
    case (7)
      {A(c,'a','K',1)}
      {DF(c)}
    endcase
  end
endmodule""")
    c = f'NA0{k}S'
    na(c, f'K|scrutinee K, gen-if shadow {d}', f"""
module top;
  {o}
  if (1) begin : gb
    {i}
    case (K)
      {A(c,'seven',"65'd7",1)}
      {A(c,'five',"65'd5",2)}
      {DF(c)}
    endcase
  end
endmodule""")
    c = f'NA0{k}F'
    na(c, f'7|label K forward ref (decl after case), shadow {d}', f"""
module top;
  {o}
  if (1) begin : gb
    case (7)
      {A(c,'a','K',1)}
      {DF(c)}
    endcase
    {i}
  end
endmodule""")
c='NA06L'
na(c, '1|label i: genvar under same-named wide constant', f"""
module top;
  localparam [64:0] i = 65'd9;
  for (genvar i = 0; i < 2; i++) begin : g
    case (1)
      {A(c,'a','i',1)}
      {DF(c)}
    endcase
  end
endmodule""")
c='NA06S'
na(c, 'i|scrutinee i: genvar under same-named wide constant', f"""
module top;
  localparam [64:0] i = 65'd9;
  for (genvar i = 0; i < 2; i++) begin : g
    case (i)
      {A(c,'nine',"65'd9",1)}
      {A(c,'one',"65'd1",2)}
      {DF(c)}
    endcase
  end
endmodule""")
c='NA07L'
na(c, '1|label K: sibling gen blocks with same-named wide localparams', f"""
module top;
  if (1) begin : b1
    localparam [64:0] K = 65'd1;
    case (1)
      {A(c,'b1a','K',1)}
      {DF(c)}
    endcase
  end
  if (1) begin : b2
    localparam [64:0] K = 65'd2;
    case (1)
      {A(c,'b2a','K',2)}
      {A(c,'b2def','default',3)}
    endcase
  end
endmodule""")
c='NA07S'
na(c, 'K|scrutinee K: sibling gen blocks, narrow', f"""
module top;
  if (1) begin : b1
    localparam logic signed [3:0] K = -1;
    case (K)
      {A(c,'b1hit',"4'b1111",1)}
      {A(c,'b1def','default',2)}
    endcase
  end
  if (1) begin : b2
    localparam logic [3:0] K = 4'd15;
    case (K)
      {A(c,'b2hit',"-1",3)}
      {A(c,'b2def','default',4)}
    endcase
  end
endmodule""")
c='NA08L'
na(c, '1|label L=i*2+j (65-bit) nested loops', f"""
module top;
  for (genvar i = 0; i < 2; i++) begin : gi
    for (genvar j = 0; j < 2; j++) begin : gj
      localparam [64:0] L = i * 2 + j;
      case (1)
        L: begin : a wire [7:0] w = 8'd1; initial #1 $display("NA08L a_%0d%0d %0d", i, j, w); end
        default: begin : d wire [7:0] w = 8'd9; initial #1 $display("NA08L d_%0d%0d %0d", i, j, w); end
      endcase
    end
  end
endmodule""")
c='NA08S'
na(c, 'i*2+j|scrutinee nested loops vs 65-bit labels', f"""
module top;
  for (genvar i = 0; i < 2; i++) begin : gi
    for (genvar j = 0; j < 2; j++) begin : gj
      case (i * 2 + j)
        65'd1: begin : a wire [7:0] w = 8'd1; initial #1 $display("NA08S one_%0d%0d %0d", i, j, w); end
        65'd2: begin : b wire [7:0] w = 8'd2; initial #1 $display("NA08S two_%0d%0d %0d", i, j, w); end
        default: begin : d wire [7:0] w = 8'd9; initial #1 $display("NA08S d_%0d%0d %0d", i, j, w); end
      endcase
    end
  end
endmodule""")
c='NA09L'
na(c, '7|label K: gen-scope variable shadowing a module param (illegal)', f"""
module top;
  localparam [64:0] K = 65'd7;
  if (1) begin : gb
    logic [64:0] K;
    case (7)
      {A(c,'a','K',1)}
      {DF(c)}
    endcase
  end
endmodule""")
c='NA09S'
na(c, 'K|scrutinee K: gen-scope variable shadowing a module param (illegal)', f"""
module top;
  localparam logic signed [3:0] K = -1;
  if (1) begin : gb
    logic [3:0] K;
    case (K)
      {A(c,'hit',"4'b1111",1)}
      {DF(c)}
    endcase
  end
endmodule""")
c='NA10L'
na(c, '7|label K: block wildcard import beside module K', f"""
package pk; localparam [64:0] K = 65'd7; endpackage
module top;
  localparam [64:0] K = 65'd5;
  if (1) begin : gb
    import pk::*;
    case (7)
      {A(c,'a','K',1)}
      {DF(c)}
    endcase
  end
endmodule""")
c='NA10E'
na(c, '7|label K: block explicit import pk::K (no module K)', f"""
package pk; localparam [64:0] K = 65'd7; endpackage
module top;
  if (1) begin : gb
    import pk::K;
    case (7)
      {A(c,'a','K',1)}
      {DF(c)}
    endcase
  end
endmodule""")
c='NA10T'
na(c, '7|label K: block let K beside module K', f"""
module top;
  localparam [64:0] K = 65'd5;
  if (1) begin : gb
    let K = 65'd7;
    case (7)
      {A(c,'a','K',1)}
      {DF(c)}
    endcase
  end
endmodule""")
c='NA11M'
na(c, "4'b1111|label EM: module enum signed label", f"""
module top;
  typedef enum logic signed [3:0] {{EM = -1, EZ = 0}} e_t;
  case (4'b1111)
    {A(c,'a','EM',1)}
    {DF(c)}
  endcase
endmodule""")
c='NA11G'
na(c, "4'b1111|label EM: gen-scope enum beside module localparam EM", f"""
module top;
  localparam logic [3:0] EM = 4'd5;
  if (1) begin : gb
    typedef enum logic signed [3:0] {{EM = -1, EZ = 0}} e_t;
    case (4'b1111)
      {A(c,'a','EM',1)}
      {DF(c)}
    endcase
  end
endmodule""")
c='NA11S'
na(c, "EM|scrutinee EM: gen-scope enum", f"""
module top;
  localparam logic [3:0] EM = 4'd5;
  if (1) begin : gb
    typedef enum logic signed [3:0] {{EM = -1, EZ = 0}} e_t;
    case (EM)
      {A(c,'hit',"4'b1111",1)}
      {A(c,'five',"4'd5",2)}
      {DF(c)}
    endcase
  end
endmodule""")
c='NA12'
na(c, 'P|instance-array element: sub case(1) P, top same-named P', f"""
module sub #(parameter [64:0] P = 0);
  case (1)
    P: begin : a wire [7:0] w = 8'd1; initial #1 $display("NA12 a %m %0d", w); end
    default: begin : d wire [7:0] w = 8'd9; initial #1 $display("NA12 d %m %0d", w); end
  endcase
endmodule
module top;
  localparam [64:0] P = 65'd0;
  sub #(.P(65'd1)) ua[1:0] ();
endmodule""")
c='NA12S'
na(c, 'P|instance-array element: sub case(P) signed narrow', f"""
module sub #(parameter logic signed [3:0] P = 0);
  case (P)
    4'b1111: begin : a wire [7:0] w = 8'd1; initial #1 $display("NA12S a %m %0d", w); end
    default: begin : d wire [7:0] w = 8'd9; initial #1 $display("NA12S d %m %0d", w); end
  endcase
endmodule
module top;
  localparam logic [3:0] P = 4'd0;
  sub #(.P(-1)) ua[1:0] ();
endmodule""")
c='NA13L'
na(c, '7|label pk::K beside module K', f"""
package pk; localparam [64:0] K = 65'd7; endpackage
module top;
  localparam [64:0] K = 65'd5;
  case (7)
    {A(c,'a','pk::K',1)}
    {DF(c)}
  endcase
  case (5)
    {A(c,'b','K',2)}
    {A(c,'bdef','default',3)}
  endcase
endmodule""")
c='NA13I'
na(c, '5|label K: module K beside wildcard-imported pk::K (local wins)', f"""
package pk; localparam [64:0] K = 65'd7; endpackage
module top;
  import pk::*;
  localparam [64:0] K = 65'd5;
  case (5)
    {A(c,'a','K',1)}
    {DF(c)}
  endcase
endmodule""")
c='NA13S'
na(c, 'pk::K|scrutinee pk::KS signed beside module KS', f"""
package pk; localparam logic signed [3:0] KS = -1; endpackage
module top;
  localparam logic [3:0] KS = 4'd5;
  case (pk::KS)
    {A(c,'hit',"4'b1111",1)}
    {A(c,'five',"4'd5",2)}
    {DF(c)}
  endcase
endmodule""")

# ---- residue twins: the same unreadable label whose true value does NOT equal the scrutinee
std('L07T', "16'h6164", [('S1 + 1', 'a')], 'localparam S1 = "ab";')
std('L13T', '2', [('R', 'a')], 'localparam real R = 1.0;')
std('L14T', '4', [('top.P', 'a')], 'localparam P = 3;')
std('X05T', '1', [("4'b1x00 & 4'b0011", 'a')])
std('X20T', '0', [("1'bx ? 1 : 1", 'a')])
std('L12T', '2', [('1.0', 'a')])

meta = {}
for (cid, S, desc, text) in cells:
    open(f'{D}/{cid}.sv', 'w').write(text)
    meta[cid] = {'scrut': S, 'label': desc}
json.dump(meta, open(D + '/meta.json', 'w'), indent=1)
print(len(cells), 'cells')
